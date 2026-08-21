use crate::error::{Error, Span};
use crate::expr::{Binding, Direction, Document, Entry, Expr, ExprKind, Field, Form};
use crate::lexer::{Lexer, TokenKind};
use crate::surface;

pub const TABSTOP: usize = 8;

pub const WIDTH: usize = 100;

pub fn format(source: &str) -> Result<String, Error> {
    let document = surface::parse(source)?;
    let mut printer = Printer::new(source, comments(source)?);
    printer.document(&document);
    Ok(printer.finish())
}

fn numeric(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::Integer(_) | ExprKind::Float(_))
}

fn comments(source: &str) -> Result<Vec<Span>, Error> {
    Lexer::new(source)
        .filter_map(|token| match token {
            Ok(token) if token.kind == TokenKind::Comment => Some(Ok(token.span)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    Inline,
    Author,
    Reflow,
}

enum Item<'e> {
    Binding(&'e Binding),
    Field(&'e Field),
    Entry(&'e Entry),
    Value(&'e Expr),
}

impl Item<'_> {
    const fn start(&self) -> usize {
        match self {
            Self::Binding(binding) => binding.name.span.start,
            Self::Field(field) => field.name.span.start,
            Self::Entry(entry) => entry.key.span.start,
            Self::Value(expr) => expr.span.start,
        }
    }
}

struct Printer<'a> {
    source: &'a str,
    comments: Vec<Span>,
    next: usize,
    out: String,
    last: usize,
    broken: bool,
}

impl<'a> Printer<'a> {
    fn new(source: &'a str, comments: Vec<Span>) -> Self {
        Self {
            source,
            comments,
            next: 0,
            out: String::with_capacity(source.len()),
            last: 0,
            broken: false,
        }
    }

    fn text(&self, span: Span) -> &'a str {
        &self.source[span.start..span.end]
    }

    fn gap(&self, until: usize) -> &'a str {
        self.source.get(self.last..until).unwrap_or_default()
    }

    fn column(&self) -> usize {
        let start = self.out.rfind('\n').map_or(0, |at| at + 1);
        self.out[start..]
            .chars()
            .map(|character| if character == '\t' { TABSTOP } else { 1 })
            .sum()
    }

    fn push(&mut self, text: &str) {
        self.out.push_str(text);
    }

    fn line(&mut self, indent: usize) {
        while self.out.ends_with([' ', '\t']) {
            self.out.pop();
        }
        self.out.push('\n');
        for _ in 0..indent {
            self.out.push('\t');
        }
        self.broken = false;
    }

    fn blank(&mut self, indent: usize) {
        self.line(indent);
        let at = self.out.len() - indent;
        self.out.insert(at, '\n');
    }

    fn flush(&mut self, before: usize, indent: usize) {
        while let Some(span) = self
            .comments
            .get(self.next)
            .copied()
            .filter(|span| span.start < before)
        {
            self.next += 1;
            let gap = self.gap(span.start);
            if self.out.is_empty() {
                self.push(self.text(span).trim_end());
            } else {
                match gap.matches('\n').count() {
                    0 => self.push(" "),
                    1 => self.line(indent),
                    _ => self.blank(indent),
                }
                let text = self.text(span).trim_end();
                self.push(text);
            }
            self.last = span.end;
            self.broken = true;
        }
    }

    fn separate(&mut self, start: usize, indent: usize, force: bool) {
        if self.out.is_empty() {
            return;
        }
        match self.gap(start).matches('\n').count() {
            0 if force => self.line(indent),
            0 => {
                if !self.out.ends_with(char::is_whitespace) {
                    self.push(" ");
                }
            }
            1 => self.line(indent),
            _ => self.blank(indent),
        }
    }

    fn lead(&mut self, start: usize, indent: usize, force: bool) {
        self.flush(start, indent);
        self.separate(start, indent, force);
    }

    fn operator(&self, symbol: &str, until: usize) -> usize {
        let mut at = self.last;
        while at < until {
            if let Some(comment) = self
                .comments
                .iter()
                .find(|comment| comment.start <= at && at < comment.end)
            {
                at = comment.end;
                continue;
            }
            if self.source[at..].starts_with(symbol) {
                return at;
            }
            at += self.source[at..].chars().next().map_or(1, char::len_utf8);
        }
        until
    }

    fn binder(&mut self, symbol: &str, before: usize, indent: usize) {
        let at = self.operator(symbol, before);
        self.flush(at, indent);
        if self.broken {
            self.line(indent);
        } else {
            self.push(" ");
        }
        self.push(symbol);
        self.last = at + symbol.len();
    }

    fn after(&mut self, expr: &Expr, indent: usize) {
        self.flush(expr.span.start, indent);
        if self.broken {
            self.line(indent);
        } else {
            self.push(" ");
        }
        self.expr(expr, indent);
        self.last = expr.span.end;
    }

    fn document(&mut self, document: &Document) {
        let mut items: Vec<Item<'_>> = document.bindings.iter().map(Item::Binding).collect();
        match (document.form, &document.value.kind) {
            (Form::Fields, ExprKind::Tuple { fields, .. }) => {
                items.extend(fields.iter().map(Item::Field));
            }
            _ => items.push(Item::Value(&document.value)),
        }
        for item in &items {
            self.lead(item.start(), 0, false);
            self.emit(item, 0);
        }
        self.flush(self.source.len(), 0);
    }

    fn emit(&mut self, item: &Item<'_>, indent: usize) {
        match item {
            Item::Binding(binding) => {
                let text = self.text(binding.name.span);
                self.push(text);
                self.last = binding.name.span.end;
                self.binder(":=", binding.value.span.start, indent);
                self.after(&binding.value, indent);
            }
            Item::Field(field) => {
                let text = self.text(field.name.span);
                self.push(text);
                self.last = field.name.span.end;
                self.binder("=", field.value.span.start, indent);
                self.after(&field.value, indent);
            }
            Item::Entry(entry) => {
                self.expr(&entry.key, indent);
                self.last = entry.key.span.end;
                self.binder("=>", entry.value.span.start, indent);
                self.after(&entry.value, indent);
            }
            Item::Value(expr) => {
                self.expr(expr, indent);
                self.last = expr.span.end;
            }
        }
    }

    fn expr(&mut self, expr: &Expr, indent: usize) {
        match &expr.kind {
            ExprKind::Tuple { bindings, fields } => {
                let mut items: Vec<Item<'_>> = bindings.iter().map(Item::Binding).collect();
                items.extend(fields.iter().map(Item::Field));
                self.block(expr.span, '{', '}', &items, indent);
            }
            ExprKind::Map { bindings, entries } => {
                let mut items: Vec<Item<'_>> = bindings.iter().map(Item::Binding).collect();
                items.extend(entries.iter().map(Item::Entry));
                self.block(expr.span, '{', '}', &items, indent);
            }
            ExprKind::List(values) => {
                let items: Vec<Item<'_>> = values.iter().map(Item::Value).collect();
                self.block(expr.span, '[', ']', &items, indent);
            }
            ExprKind::Access { operand, field } => {
                self.expr(operand, indent);
                self.last = operand.span.end;
                self.dot(operand, field.span.start, indent);
                let text = self.text(field.span);
                self.push(text);
                self.last = field.span.end;
            }
            ExprKind::Index { operand, key } => {
                self.expr(operand, indent);
                self.last = operand.span.end;
                self.dot(operand, key.span.start, indent);
                self.push("(");
                self.expr(key, indent);
                self.last = key.span.end;
                self.push(")");
            }
            ExprKind::Apply {
                function,
                argument,
                direction,
            } => {
                let (left, right, symbol) = match direction {
                    Direction::Backward => (function, argument, "<|"),
                    Direction::Forward => (argument, function, "|>"),
                };
                self.expr(left, indent);
                self.last = left.span.end;
                self.binder(symbol, right.span.start, indent);
                self.after(right, indent);
            }
            ExprKind::Group(inner) => {
                self.push("(");
                self.last = expr.span.start + 1;
                self.expr(inner, indent);
                self.last = inner.span.end;
                self.flush(expr.span.end, indent);
                self.push(")");
                self.last = expr.span.end;
            }
            ExprKind::Builtin(name) => {
                self.push("@");
                let text = self.text(name.span);
                self.push(text);
                self.last = name.span.end;
            }
            _ => {
                let text = self.text(expr.span);
                self.push(text);
                self.last = expr.span.end;
            }
        }
    }

    fn dot(&mut self, operand: &Expr, before: usize, indent: usize) {
        self.flush(before, indent);
        if self.broken {
            self.line(indent);
        } else if numeric(operand) {
            self.push(" ");
        }
        self.push(".");
    }

    fn block(&mut self, span: Span, open: char, close: char, items: &[Item<'_>], indent: usize) {
        let layout = self.layout(span, items, indent);
        self.out.push(open);
        self.last = span.start + open.len_utf8();
        if layout == Layout::Inline {
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    self.push(" ");
                }
                self.emit(item, indent);
            }
        } else {
            let inner = indent + 1;
            for (at, item) in items.iter().enumerate() {
                self.lead(item.start(), inner, at == 0 || layout == Layout::Reflow);
                self.emit(item, inner);
            }
            self.flush(span.end, inner);
            self.line(indent);
        }
        self.out.push(close);
        self.last = span.end;
    }

    fn layout(&self, span: Span, items: &[Item<'_>], indent: usize) -> Layout {
        if self
            .comments
            .get(self.next)
            .is_some_and(|comment| comment.start < span.end)
        {
            return Layout::Author;
        }
        if items.is_empty() {
            return Layout::Inline;
        }
        let inner = Span::new(span.start + 1, span.end - 1);
        if self.text(inner).contains('\n') {
            return Layout::Author;
        }
        if self.column() + self.measure(items, indent) + 2 > WIDTH {
            return Layout::Reflow;
        }
        Layout::Inline
    }

    fn measure(&self, items: &[Item<'_>], indent: usize) -> usize {
        let mut scratch = Printer::new(self.source, Vec::new());
        for (at, item) in items.iter().enumerate() {
            if at > 0 {
                scratch.push(" ");
            }
            scratch.emit(item, indent);
        }
        scratch.out.chars().count()
    }

    fn finish(mut self) -> String {
        while self.out.ends_with(char::is_whitespace) {
            self.out.pop();
        }
        self.out.push('\n');
        self.out
    }
}
