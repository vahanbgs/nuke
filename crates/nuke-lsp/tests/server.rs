use std::fs;
use std::path::Path;
use std::thread::JoinHandle;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::Notification as _;
use lsp_types::request::Request as _;
use lsp_types::{
    ClientCapabilities, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, DocumentFormattingParams, FormattingOptions,
    GeneralClientCapabilities, GotoDefinitionParams, InitializeParams, InitializeResult,
    InitializedParams, Position, PositionEncodingKind, PublishDiagnosticsParams, Range,
    ReferenceContext, ReferenceParams, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, TextEdit, Url, VersionedTextDocumentIdentifier,
    WorkDoneProgressParams, notification, request,
};
use serde_json::{Value, json};
use tempfile::TempDir;

const LANGUAGE: &str = "nuke";

const WAIT: Duration = Duration::from_secs(10);

#[test]
fn initialize_advertises_what_the_server_answers() {
    let mut client = Client::start();
    let capabilities = client.capabilities.clone();
    assert_eq!(capabilities["positionEncoding"], json!("utf-16"));
    assert_eq!(capabilities["definitionProvider"], json!(true));
    assert_eq!(capabilities["referencesProvider"], json!(true));
    assert_eq!(capabilities["documentHighlightProvider"], json!(true));
    assert_eq!(capabilities["documentSymbolProvider"], json!(true));
    assert_eq!(capabilities["documentFormattingProvider"], json!(true));
    assert_eq!(capabilities["textDocumentSync"]["change"], json!(1));
    client.stop();
}

#[test]
fn a_style_fault_and_an_unbound_name_arrive_together() {
    let mut client = Client::start();
    let uri = client.open("unused := 1\n[missing]");
    let published = client.published();
    assert_eq!(published.uri, uri);
    assert_eq!(published.diagnostics.len(), 2, "{published:?}");
    let style = &published.diagnostics[0];
    assert_eq!(style.severity, Some(DiagnosticSeverity::WARNING));
    assert_eq!(
        style.code,
        Some(lsp_types::NumberOrString::String(
            "unused-binding".to_owned()
        ))
    );
    assert_eq!(style.range, at(0, 0, 0, 6));
    let fault = &published.diagnostics[1];
    assert_eq!(fault.severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(fault.code, None);
    assert_eq!(fault.range, at(1, 1, 1, 8));
    assert!(fault.message.contains("not bound here"), "{fault:?}");
    client.stop();
}

#[test]
fn a_document_that_does_not_parse_has_one_fault_and_will_not_format() {
    let mut client = Client::start();
    let uri = client.open("{a = ");
    let published = client.published();
    assert_eq!(published.diagnostics.len(), 1, "{published:?}");
    assert_eq!(
        published.diagnostics[0].severity,
        Some(DiagnosticSeverity::ERROR)
    );
    let response = client.request(request::Formatting::METHOD, formatting(&uri));
    assert!(response.result.is_none(), "{response:?}");
    assert!(response.error.is_some(), "{response:?}");
    client.stop();
}

#[test]
fn a_name_is_followed_to_the_binding_it_reads() {
    let mut client = Client::start();
    let uri = client.open("colour := \"red\"\n{a = colour b = colour}");
    let _ = client.published();
    let found = client.request(
        request::GotoDefinition::METHOD,
        serde_json::to_value(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(1, 5),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: lsp_types::PartialResultParams::default(),
        })
        .expect("params"),
    );
    let location = found.result.expect("a location");
    assert_eq!(
        location["range"],
        serde_json::to_value(at(0, 0, 0, 6)).unwrap()
    );
    let found = client.request(
        request::References::METHOD,
        serde_json::to_value(ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(1, 5),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: lsp_types::PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        })
        .expect("params"),
    );
    let locations = found.result.expect("locations");
    assert_eq!(
        locations.as_array().map(Vec::len),
        Some(3),
        "the binding and both reads: {locations}"
    );
    client.stop();
}

#[test]
fn formatting_answers_one_edit_or_none_at_all() {
    let mut client = Client::start();
    let uri = client.open("{a:=1 b = a}");
    let _ = client.published();
    let response = client.request(request::Formatting::METHOD, formatting(&uri));
    let edits: Vec<TextEdit> =
        serde_json::from_value(response.result.expect("edits")).expect("edits");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "{a := 1 b = a}\n");

    client.change(&uri, "{a := 1 b = a}\n");
    let _ = client.published();
    let response = client.request(request::Formatting::METHOD, formatting(&uri));
    let edits: Vec<TextEdit> =
        serde_json::from_value(response.result.expect("edits")).expect("edits");
    assert!(edits.is_empty(), "{edits:?}");
    client.stop();
}

#[test]
fn an_outline_names_the_bindings_and_the_fields() {
    let mut client = Client::start();
    let uri = client.open("size := 12\n{font = {family = \"Iosevka\" size = size}}");
    let _ = client.published();
    let response = client.request(
        request::DocumentSymbolRequest::METHOD,
        json!({ "textDocument": { "uri": uri } }),
    );
    let symbols = response.result.expect("symbols");
    assert_eq!(symbols[0]["name"], json!("size"));
    assert_eq!(symbols[1]["name"], json!("font"));
    assert_eq!(symbols[1]["children"][0]["name"], json!("family"));
    assert_eq!(symbols[1]["children"][1]["name"], json!("size"));
    client.stop();
}

#[test]
fn an_unknown_request_is_refused_rather_than_ignored() {
    let mut client = Client::start();
    let response = client.request("textDocument/inlayHint", json!({}));
    assert!(response.error.is_some(), "{response:?}");
    client.stop();
}

#[test]
fn an_imported_fault_is_reported_at_the_import_that_asked_for_it() {
    let directory = TempDir::new().expect("a directory");
    let theme = directory.path().join("theme.nuke");
    fs::write(&theme, "{colour = \"red\"}\n").expect("written");
    let mut client = Client::start();
    let uri = client.open_at(
        &directory.path().join("main.nuke"),
        "theme := @import <| \"./theme.nuke\"\n{a = theme.shade}",
    );
    let published = client.published();
    assert_eq!(published.diagnostics.len(), 1, "{published:?}");
    assert!(
        published.diagnostics[0]
            .message
            .contains("no field `shade`"),
        "{published:?}"
    );

    client.change(
        &uri,
        "theme := @import <| \"./theme.nuke\"\n{a = theme.colour}",
    );
    let published = client.published();
    assert!(published.diagnostics.is_empty(), "{published:?}");

    fs::write(&theme, "{colour = shade}\n").expect("written");
    client.save(&theme);
    let published = client.published();
    assert_eq!(published.uri, uri, "the importer is re-diagnosed");
    assert_eq!(published.diagnostics.len(), 1, "{published:?}");
    let fault = &published.diagnostics[0];
    assert_eq!(fault.range, at(0, 9, 0, 34), "the `@import` that asked");
    let related = fault.related_information.as_ref().expect("a related fault");
    assert_eq!(related.len(), 1);
    assert!(
        related[0]
            .location
            .uri
            .to_file_path()
            .expect("a path")
            .ends_with("theme.nuke"),
        "{related:?}"
    );
    assert_eq!(related[0].location.range.start, Position::new(0, 10));
    client.stop();
}

#[test]
fn an_offered_encoding_is_taken_and_a_wide_character_is_counted() {
    let mut client = Client::start();
    let _ = client.open("{a = \"\u{1f3a8}\" b = missing}");
    let published = client.published();
    assert_eq!(
        published.diagnostics[0].range.start,
        Position::new(0, 14),
        "one emoji is two utf-16 units"
    );
    client.stop();

    let mut client = Client::offering(PositionEncodingKind::UTF8);
    assert_eq!(client.capabilities["positionEncoding"], json!("utf-8"));
    let _ = client.open("{a = \"\u{1f3a8}\" b = missing}");
    let published = client.published();
    assert_eq!(
        published.diagnostics[0].range.start,
        Position::new(0, 16),
        "one emoji is four utf-8 units"
    );
    client.stop();
}

fn at(line: u32, start: u32, end_line: u32, end: u32) -> Range {
    Range::new(Position::new(line, start), Position::new(end_line, end))
}

fn formatting(uri: &Url) -> Value {
    serde_json::to_value(DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        options: FormattingOptions::default(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    })
    .expect("params")
}

struct Client {
    connection: Connection,
    served: Option<JoinHandle<Result<(), nuke_lsp::Error>>>,
    capabilities: Value,
    next: i32,
    directory: TempDir,
}

impl Client {
    fn start() -> Self {
        Self::with(InitializeParams::default())
    }

    fn offering(encoding: PositionEncodingKind) -> Self {
        Self::with(InitializeParams {
            capabilities: ClientCapabilities {
                general: Some(GeneralClientCapabilities {
                    position_encodings: Some(vec![encoding]),
                    ..GeneralClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        })
    }

    fn with(initialize: InitializeParams) -> Self {
        let (server, client) = Connection::memory();
        let served = std::thread::spawn(move || nuke_lsp::serve_on(&server));
        let id = RequestId::from(0);
        client
            .sender
            .send(Message::Request(Request::new(
                id.clone(),
                request::Initialize::METHOD.to_owned(),
                initialize,
            )))
            .expect("sent");
        let Message::Response(response) = client.receiver.recv_timeout(WAIT).expect("a response")
        else {
            panic!("the server answered initialize with something else");
        };
        assert_eq!(response.id, id);
        client
            .sender
            .send(Message::Notification(Notification::new(
                notification::Initialized::METHOD.to_owned(),
                InitializedParams {},
            )))
            .expect("sent");
        let result: InitializeResult =
            serde_json::from_value(response.result.expect("a result")).expect("a result");
        Self {
            connection: client,
            served: Some(served),
            capabilities: serde_json::to_value(result.capabilities).expect("capabilities"),
            next: 1,
            directory: TempDir::new().expect("a directory"),
        }
    }

    fn open(&mut self, text: &str) -> Url {
        let path = self.directory.path().join("document.nuke");
        self.open_at(&path, text)
    }

    fn open_at(&mut self, path: &Path, text: &str) -> Url {
        let uri = Url::from_file_path(path).expect("a file url");
        self.notify(
            notification::DidOpenTextDocument::METHOD,
            serde_json::to_value(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: LANGUAGE.to_owned(),
                    version: 1,
                    text: text.to_owned(),
                },
            })
            .expect("params"),
        );
        uri
    }

    fn change(&mut self, uri: &Url, text: &str) {
        self.notify(
            notification::DidChangeTextDocument::METHOD,
            serde_json::to_value(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: text.to_owned(),
                }],
            })
            .expect("params"),
        );
    }

    fn save(&mut self, path: &Path) {
        self.notify(
            notification::DidSaveTextDocument::METHOD,
            serde_json::to_value(DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(path).expect("a file url"),
                },
                text: None,
            })
            .expect("params"),
        );
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.connection
            .sender
            .send(Message::Notification(Notification {
                method: method.to_owned(),
                params,
            }))
            .expect("sent");
    }

    fn request(&mut self, method: &str, params: Value) -> Response {
        let id = RequestId::from(self.next);
        self.next += 1;
        self.connection
            .sender
            .send(Message::Request(Request {
                id: id.clone(),
                method: method.to_owned(),
                params,
            }))
            .expect("sent");
        loop {
            match self
                .connection
                .receiver
                .recv_timeout(WAIT)
                .expect("a reply")
            {
                Message::Response(response) if response.id == id => return response,
                Message::Notification(_) | Message::Response(_) => {}
                Message::Request(request) => panic!("the server asked for {}", request.method),
            }
        }
    }

    fn published(&mut self) -> PublishDiagnosticsParams {
        loop {
            match self
                .connection
                .receiver
                .recv_timeout(WAIT)
                .expect("diagnostics")
            {
                Message::Notification(notification)
                    if notification.method == notification::PublishDiagnostics::METHOD =>
                {
                    return serde_json::from_value(notification.params).expect("diagnostics");
                }
                _ => {}
            }
        }
    }

    fn stop(&mut self) {
        let id = RequestId::from(self.next);
        self.next += 1;
        self.connection
            .sender
            .send(Message::Request(Request::new(
                id,
                request::Shutdown::METHOD.to_owned(),
                Value::Null,
            )))
            .expect("sent");
        self.notify(notification::Exit::METHOD, Value::Null);
        self.served
            .take()
            .expect("a server")
            .join()
            .expect("joined")
            .expect("served");
    }
}
