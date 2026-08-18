use std::collections::HashMap;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::Notification as _;
use lsp_types::request::Request as _;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, DocumentFormattingParams, DocumentHighlightParams,
    DocumentSymbolParams, GotoDefinitionParams, GotoDefinitionResponse, InitializeParams,
    InitializeResult, Location, OneOf, PublishDiagnosticsParams, ReferenceParams,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, TextEdit, Url, error_codes, notification,
    request,
};
use serde::Serialize;
use serde_json::Value;

use crate::document::Open;
use crate::position::{self, Encoding};
use crate::{Error, diagnostics, navigate};

const NAME: &str = "nuke";

pub(crate) fn serve(connection: &Connection) -> Result<(), Error> {
    let (id, params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(params)?;
    let encoding = Encoding::offered(
        params
            .capabilities
            .general
            .as_ref()
            .and_then(|general| general.position_encodings.as_deref()),
    );
    let result = InitializeResult {
        capabilities: capabilities(encoding),
        server_info: Some(ServerInfo {
            name: NAME.to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    };
    connection.initialize_finish(id, serde_json::to_value(result)?)?;
    Server {
        encoding,
        open: HashMap::new(),
    }
    .run(connection)
}

struct Server {
    encoding: Encoding,
    open: HashMap<Url, Open>,
}

struct Fault {
    code: i32,
    message: String,
}

impl Server {
    fn run(mut self, connection: &Connection) -> Result<(), Error> {
        for message in &connection.receiver {
            match message {
                Message::Request(request) => {
                    if connection.handle_shutdown(&request)? {
                        return Ok(());
                    }
                    let response = self.request(request);
                    send(connection, Message::Response(response))?;
                }
                Message::Notification(notification) => {
                    self.notification(connection, notification)?;
                }
                Message::Response(_) => {}
            }
        }
        Ok(())
    }

    fn request(&self, request: Request) -> Response {
        let Request { id, method, params } = request;
        let answered = match method.as_str() {
            request::GotoDefinition::METHOD => self.definition(params),
            request::References::METHOD => self.references(params),
            request::DocumentHighlightRequest::METHOD => self.highlights(params),
            request::DocumentSymbolRequest::METHOD => self.symbols(params),
            request::Formatting::METHOD => self.formatting(params),
            _ => Err(Fault {
                code: ErrorCode::MethodNotFound as i32,
                message: format!("`{method}` is not a request this server answers"),
            }),
        };
        match answered {
            Ok(value) => Response::new_ok(id, value),
            Err(fault) => Response::new_err(id, fault.code, fault.message),
        }
    }

    fn definition(&self, params: Value) -> Result<Value, Fault> {
        let params: GotoDefinitionParams = taken(params)?;
        let at = &params.text_document_position_params;
        let Some((open, offset)) = self.located(&at.text_document.uri, at.position) else {
            return answer(&Value::Null);
        };
        let Some(span) = navigate::definition(open, offset) else {
            return answer(&Value::Null);
        };
        answer(&GotoDefinitionResponse::Scalar(Location {
            uri: at.text_document.uri.clone(),
            range: position::range(&open.index, self.encoding, span),
        }))
    }

    fn references(&self, params: Value) -> Result<Value, Fault> {
        let params: ReferenceParams = taken(params)?;
        let at = &params.text_document_position;
        let Some((open, offset)) = self.located(&at.text_document.uri, at.position) else {
            return answer(&Value::Null);
        };
        let found: Vec<Location> =
            navigate::references(open, offset, params.context.include_declaration)
                .into_iter()
                .map(|span| Location {
                    uri: at.text_document.uri.clone(),
                    range: position::range(&open.index, self.encoding, span),
                })
                .collect();
        answer(&found)
    }

    fn highlights(&self, params: Value) -> Result<Value, Fault> {
        let params: DocumentHighlightParams = taken(params)?;
        let at = &params.text_document_position_params;
        let Some((open, offset)) = self.located(&at.text_document.uri, at.position) else {
            return answer(&Value::Null);
        };
        answer(&navigate::highlights(open, offset, self.encoding))
    }

    fn symbols(&self, params: Value) -> Result<Value, Fault> {
        let params: DocumentSymbolParams = taken(params)?;
        let Some(open) = self.open.get(&params.text_document.uri) else {
            return answer(&Value::Null);
        };
        answer(&navigate::symbols(open, self.encoding))
    }

    fn formatting(&self, params: Value) -> Result<Value, Fault> {
        let params: DocumentFormattingParams = taken(params)?;
        let Some(open) = self.open.get(&params.text_document.uri) else {
            return answer(&Value::Null);
        };
        let formatted = nuke_syntax::printer::format(&open.text).map_err(|error| Fault {
            code: failed(),
            message: error.to_string(),
        })?;
        if formatted == open.text {
            return answer::<[TextEdit; 0]>(&[]);
        }
        answer(&[TextEdit {
            range: position::whole(&open.index, self.encoding, &open.text),
            new_text: formatted,
        }])
    }

    fn notification(
        &mut self,
        connection: &Connection,
        notification: Notification,
    ) -> Result<(), Error> {
        let Notification { method, params } = notification;
        match method.as_str() {
            notification::DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(params)?;
                let uri = params.text_document.uri;
                self.open
                    .insert(uri.clone(), Open::new(params.text_document.text));
                self.diagnose(connection, &uri)?;
            }
            notification::DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(params)?;
                let uri = params.text_document.uri;
                if let Some(change) = params.content_changes.into_iter().next_back() {
                    self.open.insert(uri.clone(), Open::new(change.text));
                    self.diagnose(connection, &uri)?;
                }
            }
            notification::DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams = serde_json::from_value(params)?;
                for uri in self.dependents(&params.text_document.uri) {
                    self.diagnose(connection, &uri)?;
                }
            }
            notification::DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(params)?;
                let uri = params.text_document.uri;
                self.open.remove(&uri);
                publish(connection, uri, Vec::new())?;
            }
            _ => {}
        }
        Ok(())
    }

    fn dependents(&self, saved: &Url) -> Vec<Url> {
        let path = saved
            .to_file_path()
            .ok()
            .map(|path| path.canonicalize().unwrap_or(path));
        self.open
            .iter()
            .filter(|(uri, open)| {
                *uri == saved
                    || !open.reduced
                    || path.as_ref().is_some_and(|path| open.read.contains(path))
            })
            .map(|(uri, _)| uri.clone())
            .collect()
    }

    fn diagnose(&mut self, connection: &Connection, uri: &Url) -> Result<(), Error> {
        let path = uri.to_file_path().ok();
        let Some(report) = self
            .open
            .get(uri)
            .map(|open| diagnostics::of(open, path.as_deref(), self.encoding))
        else {
            return Ok(());
        };
        if let Some(open) = self.open.get_mut(uri) {
            open.read = report.read;
            open.reduced = report.reduced;
        }
        publish(connection, uri.clone(), report.found)
    }

    fn located(&self, uri: &Url, position: lsp_types::Position) -> Option<(&Open, usize)> {
        let open = self.open.get(uri)?;
        let offset = position::offset(&open.index, self.encoding, position)?;
        Some((open, offset))
    }
}

fn capabilities(encoding: Encoding) -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(encoding.kind()),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                ..TextDocumentSyncOptions::default()
            },
        )),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        ..ServerCapabilities::default()
    }
}

fn publish(
    connection: &Connection,
    uri: Url,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<(), Error> {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    send(
        connection,
        Message::Notification(Notification::new(
            notification::PublishDiagnostics::METHOD.to_owned(),
            params,
        )),
    )
}

fn send(connection: &Connection, message: Message) -> Result<(), Error> {
    connection
        .sender
        .send(message)
        .map_err(|_| Error::Disconnected)
}

fn answer<T: Serialize>(value: &T) -> Result<Value, Fault> {
    serde_json::to_value(value).map_err(|error| Fault {
        code: ErrorCode::InternalError as i32,
        message: error.to_string(),
    })
}

fn taken<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, Fault> {
    serde_json::from_value(params).map_err(|error| Fault {
        code: ErrorCode::InvalidParams as i32,
        message: error.to_string(),
    })
}

fn failed() -> i32 {
    i32::try_from(error_codes::REQUEST_FAILED).unwrap_or(ErrorCode::InternalError as i32)
}
