//! LOLCODE Markdown compiler for COSC 455 Project 1.
//!
//! The program scans a `.lol` file one character at a time, validates its
//! structure with a one-token-lookahead recursive-descent parser, resolves
//! statically scoped variables, writes HTML, and opens the successful result.

// Kiara Watson
// COSC 455 Project 1 - LOLCODE Markdown compiler for macOS
// The scanner reads the source one character at a time, the parser uses one-token
// lookahead, and the translator resolves variables with a stack of static scopes.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Defines the required operations for the compiler driver.
///
/// The driver owns the current token and coordinates scanning, parsing, and
/// translation. Errors are reported by the concrete implementation.
pub trait Compiler {
    fn compile(&mut self, source: &str);
    fn next_token(&mut self) -> String;
    fn parse(&mut self);
    fn current_token(&self) -> String;
    fn set_current_token(&mut self, tok: String);
}

/// Defines the character-level operations used by the lexical analyzer.
pub trait LexicalAnalyzer {
    fn get_char(&mut self) -> char;
    fn add_char(&mut self, c: char);
    fn lookup(&self, s: &str) -> bool;
}

/// Defines one recursive-descent function for each grammar nonterminal.
pub trait SyntaxAnalyzer {
    fn parse_lolcode(&mut self) -> Result<(), String>;
    fn parse_head(&mut self) -> Result<(), String>;
    fn parse_title(&mut self) -> Result<(), String>;
    fn parse_comment(&mut self) -> Result<(), String>;
    fn parse_body(&mut self) -> Result<(), String>;
    fn parse_paragraph(&mut self) -> Result<(), String>;
    fn parse_inner_paragraph(&mut self) -> Result<(), String>;
    fn parse_inner_text(&mut self) -> Result<(), String>;
    fn parse_variable_define(&mut self) -> Result<(), String>;
    fn parse_variable_use(&mut self) -> Result<(), String>;
    fn parse_bold(&mut self) -> Result<(), String>;
    fn parse_list(&mut self) -> Result<(), String>;
    fn parse_list_items(&mut self) -> Result<(), String>;
    fn parse_inner_list(&mut self) -> Result<(), String>;
    fn parse_link(&mut self) -> Result<(), String>;
    fn parse_newline(&mut self) -> Result<(), String>;
    fn parse_text(&mut self) -> Result<(), String>;
}

// Multiword annotations are tested before shorter annotations so, for example,
// #GIMMEH TITLE is returned as one token rather than two partial lexemes.
const ANNOTATIONS: [&str; 14] = [
    "#GIMMEH PARAGRAF", // recognized only so that a helpful syntax error can be given
    "#MAEK PARAGRAF",
    "#GIMMEH TITLE",
    "#GIMMEH BOLD",
    "#GIMMEH ITEM",
    "#GIMMEH LINX",
    "#GIMMEH VAR",
    "#MAEK HEAD",
    "#MAEK LIST",
    "#NEWLINE",
    "#OBTW",
    "#TLDR",
    "#IHAZ",
    "#ITIZ",
];

const SHORT_ANNOTATIONS: [&str; 4] = ["#HAI", "#KBYE", "#MKAY", "#OIC"];

#[derive(Clone, Debug, PartialEq)]
/// The scanner deliberately uses a small token set: annotations are grammar
/// terminals, text is source content, and Eof marks the end of the input.
enum TokenKind {
    Annotation(String),
    Text(String),
    Eof,
}

#[derive(Clone, Debug)]
/// A token and its source location, retained for useful compiler errors.
struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}

/// Character-by-character scanner based on the Phase 1 state-machine design.
struct Scanner {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    current_build: String,
}

impl Scanner {
    fn new(source: &str) -> Self {
        Self {
            input: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            current_build: String::new(),
        }
    }

    fn location(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    fn starts_with_ignore_case(&self, expected: &str) -> bool {
        let chars: Vec<char> = expected.chars().collect();
        if self.position + chars.len() > self.input.len() {
            return false;
        }
        self.input[self.position..self.position + chars.len()]
            .iter()
            .zip(chars.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    }

    fn boundary_after(&self, length: usize) -> bool {
        match self.input.get(self.position + length) {
            None => true,
            Some(c) => c.is_whitespace() || c.is_ascii_punctuation(),
        }
    }

    fn scan_annotation(&mut self) -> Result<Token, String> {
        let (line, column) = self.location();
        let mut choices: Vec<&str> = ANNOTATIONS
            .iter()
            .chain(SHORT_ANNOTATIONS.iter())
            .copied()
            .collect();
        choices.sort_by_key(|item| std::cmp::Reverse(item.len()));

        // Longest-first lookup is required because several annotations share
        // prefixes such as #GIMMEH and #MAEK.
        for annotation in choices {
            let length = annotation.chars().count();
            if self.starts_with_ignore_case(annotation) && self.boundary_after(length) {
                self.current_build.clear();
                for _ in 0..length {
                    let c = self.get_char();
                    self.add_char(c);
                }
                let normalized = self.current_build.to_ascii_uppercase();
                return Ok(Token { kind: TokenKind::Annotation(normalized), line, column });
            }
        }

        // Reaching this point means a lexeme began like an annotation but did
        // not match any legal token in the lookup table.
        let mut bad = String::new();
        while let Some(c) = self.input.get(self.position).copied() {
            if c.is_whitespace() || (c == '#' && !bad.is_empty()) {
                break;
            }
            bad.push(self.get_char());
        }
        Err(format!("lexical error at {}:{}: unknown annotation '{}'", line, column, bad))
    }

    fn scan_text(&mut self) -> Token {
        let (line, column) = self.location();
        self.current_build.clear();
        // Plain text ends at '#', which the project reserves for annotations.
        while let Some(c) = self.input.get(self.position).copied() {
            if c == '#' {
                break;
            }
            let next = self.get_char();
            self.add_char(next);
        }
        Token { kind: TokenKind::Text(self.current_build.clone()), line, column }
    }

    fn next_token_result(&mut self) -> Result<Token, String> {
        if self.position >= self.input.len() {
            return Ok(Token { kind: TokenKind::Eof, line: self.line, column: self.column });
        }
        if self.input[self.position] == '#' {
            self.scan_annotation()
        } else {
            Ok(self.scan_text())
        }
    }

    fn all_tokens(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token_result()?;
            let done = token.kind == TokenKind::Eof;
            // Source formatting separates annotations but has no grammar role.
            // Whitespace surrounding actual text remains inside that text token.
            if !matches!(&token.kind, TokenKind::Text(text) if text.trim().is_empty()) {
                tokens.push(token);
            }
            if done {
                break;
            }
        }
        Ok(tokens)
    }
}

impl LexicalAnalyzer for Scanner {
    fn get_char(&mut self) -> char {
        if self.position >= self.input.len() {
            return '\0';
        }
        let c = self.input[self.position];
        self.position += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }

    fn add_char(&mut self, c: char) {
        self.current_build.push(c);
    }

    fn lookup(&self, s: &str) -> bool {
        ANNOTATIONS.iter().chain(SHORT_ANNOTATIONS.iter()).any(|item| item.eq_ignore_ascii_case(s))
    }
}

#[derive(Clone, Debug)]
/// Abstract-syntax-tree nodes. Keeping parsing separate from rendering ensures
/// that an invalid program cannot leave a partially generated HTML document.
enum Node {
    Document(Vec<Node>),
    Comment(String),
    Head(Vec<Node>),
    Title(String),
    Paragraph(Vec<Node>),
    Bold(Vec<Node>),
    List(Vec<Node>),
    Item(Vec<Node>),
    Newline,
    Link(String),
    Text(String),
    VariableDefinition { name: String, value: String },
    VariableUse(String),
}

/// Predictive recursive-descent parser using exactly one current-token lookahead.
struct Parser {
    tokens: Vec<Token>,
    position: usize,
    root: Option<Node>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, position: 0, root: None }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.position.min(self.tokens.len() - 1)]
    }

    fn annotation(&self) -> Option<&str> {
        match &self.current().kind {
            TokenKind::Annotation(value) => Some(value.as_str()),
            _ => None,
        }
    }

    fn error(&self, message: &str) -> String {
        format!("syntax error at {}:{}: {}", self.current().line, self.current().column, message)
    }

    fn advance(&mut self) {
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
    }

    fn expect_annotation(&mut self, expected: &str) -> Result<(), String> {
        if self.annotation() == Some(expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("expected {}, found {}", expected, self.describe_current())))
        }
    }

    fn describe_current(&self) -> String {
        match &self.current().kind {
            TokenKind::Annotation(value) => value.clone(),
            TokenKind::Text(value) => format!("text {:?}", value.trim()),
            TokenKind::Eof => "end of file".to_string(),
        }
    }

    fn take_text_until(&mut self, closing: &str, label: &str, allow_empty: bool) -> Result<String, String> {
        let mut value = String::new();
        loop {
            match &self.current().kind {
                TokenKind::Text(text) => {
                    value.push_str(text);
                    self.advance();
                }
                TokenKind::Annotation(tag) if tag == closing => break,
                TokenKind::Annotation(tag) => {
                    return Err(self.error(&format!("{} allows plain text only; found {}", label, tag)));
                }
                TokenKind::Eof => return Err(self.error(&format!("expected {} to close {}", closing, label))),
            }
        }
        if !allow_empty && value.trim().is_empty() {
            return Err(self.error(&format!("{} cannot be empty", label)));
        }
        Ok(value)
    }

    fn parse_document_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#HAI")?;
        let mut children = Vec::new();

        // A document-level variable definition must be the first construct.
        // Otherwise, leading comments may precede the optional head.
        if self.annotation() == Some("#IHAZ") {
            children.push(self.parse_variable_definition_node()?);
        } else {
            while self.annotation() == Some("#OBTW") {
                children.push(self.parse_comment_node()?);
            }
            if self.annotation() == Some("#MAEK HEAD") {
                children.push(self.parse_head_node()?);
            }
        }

        children.extend(self.parse_body_nodes("#KBYE", BodyContext::Document)?);
        self.expect_annotation("#KBYE")?;
        if !matches!(self.current().kind, TokenKind::Eof) {
            return Err(self.error("nothing may appear after #KBYE"));
        }
        Ok(Node::Document(children))
    }

    fn parse_head_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#MAEK HEAD")?;
        let mut children = Vec::new();
        let mut title_seen = false;
        while self.annotation() != Some("#MKAY") {
            match self.annotation() {
                Some("#OBTW") => children.push(self.parse_comment_node()?),
                Some("#GIMMEH TITLE") if !title_seen => {
                    children.push(self.parse_title_node()?);
                    title_seen = true;
                }
                Some("#GIMMEH TITLE") => return Err(self.error("a head may contain only one title")),
                Some(tag) => return Err(self.error(&format!("{} is not allowed inside the head", tag))),
                None => return Err(self.error("plain text is not allowed directly inside the head")),
            }
        }
        self.expect_annotation("#MKAY")?;
        Ok(Node::Head(children))
    }

    fn parse_title_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#GIMMEH TITLE")?;
        let text = self.take_text_until("#OIC", "title", false)?;
        self.expect_annotation("#OIC")?;
        Ok(Node::Title(text))
    }

    fn parse_comment_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#OBTW")?;
        let text = self.take_text_until("#TLDR", "comment", true)?;
        self.expect_annotation("#TLDR")?;
        Ok(Node::Comment(text))
    }

    fn parse_paragraph_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#MAEK PARAGRAF")?;
        let mut children = Vec::new();
        // The grammar permits one definition only at the start of a block.
        if self.annotation() == Some("#IHAZ") {
            children.push(self.parse_variable_definition_node()?);
        }
        children.extend(self.parse_body_nodes("#MKAY", BodyContext::Paragraph)?);
        self.expect_annotation("#MKAY")?;
        Ok(Node::Paragraph(children))
    }

    fn parse_bold_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#GIMMEH BOLD")?;
        let mut children = Vec::new();
        while self.annotation() != Some("#OIC") {
            match &self.current().kind {
                TokenKind::Text(text) => {
                    children.push(Node::Text(text.clone()));
                    self.advance();
                }
                TokenKind::Annotation(tag) if tag == "#GIMMEH VAR" => children.push(self.parse_variable_use_node()?),
                TokenKind::Annotation(tag) => return Err(self.error(&format!("{} is not allowed inside bold text", tag))),
                TokenKind::Eof => return Err(self.error("expected #OIC to close bold text")),
            }
        }
        self.expect_annotation("#OIC")?;
        Ok(Node::Bold(children))
    }

    fn parse_list_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#MAEK LIST")?;
        // A list cannot be empty and its first child must be an item.
        if self.annotation() != Some("#GIMMEH ITEM") {
            return Err(self.error("a list must begin immediately with #GIMMEH ITEM"));
        }
        let mut children = Vec::new();
        while self.annotation() != Some("#MKAY") {
            match self.annotation() {
                Some("#GIMMEH ITEM") => children.push(self.parse_item_node()?),
                Some("#OBTW") => children.push(self.parse_comment_node()?),
                Some(tag) => return Err(self.error(&format!("{} is not allowed directly inside a list", tag))),
                None => return Err(self.error("plain text must be inside a list item")),
            }
        }
        self.expect_annotation("#MKAY")?;
        Ok(Node::List(children))
    }

    fn parse_item_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#GIMMEH ITEM")?;
        let mut children = Vec::new();
        if self.annotation() == Some("#IHAZ") {
            children.push(self.parse_variable_definition_node()?);
        }
        children.extend(self.parse_body_nodes("#OIC", BodyContext::Item)?);
        self.expect_annotation("#OIC")?;
        Ok(Node::Item(children))
    }

    fn parse_link_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#GIMMEH LINX")?;
        let address = self.take_text_until("#OIC", "link", false)?;
        self.expect_annotation("#OIC")?;
        Ok(Node::Link(address.trim().to_string()))
    }

    fn parse_variable_definition_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#IHAZ")?;
        let name = match &self.current().kind {
            TokenKind::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
                    return Err(self.error("a variable name must be one word"));
                }
                let value = trimmed.to_string();
                self.advance();
                value
            }
            _ => return Err(self.error("expected a one-word variable name after #IHAZ")),
        };
        self.expect_annotation("#ITIZ")?;
        let value = self.take_text_until("#MKAY", "variable value", false)?;
        self.expect_annotation("#MKAY")?;
        Ok(Node::VariableDefinition { name, value: value.trim().to_string() })
    }

    fn parse_variable_use_node(&mut self) -> Result<Node, String> {
        self.expect_annotation("#GIMMEH VAR")?;
        let name = match &self.current().kind {
            TokenKind::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
                    return Err(self.error("a variable use must contain one variable name"));
                }
                let value = trimmed.to_string();
                self.advance();
                value
            }
            _ => return Err(self.error("expected a variable name after #GIMMEH VAR")),
        };
        self.expect_annotation("#OIC")?;
        Ok(Node::VariableUse(name))
    }

    fn parse_body_nodes(&mut self, closing: &str, context: BodyContext) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        while self.annotation() != Some(closing) {
            match &self.current().kind {
                TokenKind::Text(text) => {
                    nodes.push(Node::Text(text.clone()));
                    self.advance();
                }
                TokenKind::Annotation(tag) if tag == "#OBTW" => nodes.push(self.parse_comment_node()?),
                TokenKind::Annotation(tag) if tag == "#GIMMEH BOLD" => nodes.push(self.parse_bold_node()?),
                TokenKind::Annotation(tag) if tag == "#GIMMEH VAR" => nodes.push(self.parse_variable_use_node()?),
                TokenKind::Annotation(tag) if tag == "#NEWLINE" && context != BodyContext::Item => {
                    self.advance();
                    nodes.push(Node::Newline);
                }
                TokenKind::Annotation(tag) if tag == "#GIMMEH LINX" && context != BodyContext::Item => {
                    nodes.push(self.parse_link_node()?);
                }
                TokenKind::Annotation(tag) if tag == "#MAEK LIST" && context != BodyContext::Item => {
                    nodes.push(self.parse_list_node()?);
                }
                TokenKind::Annotation(tag) if tag == "#MAEK PARAGRAF" && context == BodyContext::Document => {
                    nodes.push(self.parse_paragraph_node()?);
                }
                TokenKind::Annotation(tag) if tag == "#GIMMEH ITEM" => {
                    return Err(self.error("#GIMMEH ITEM may appear only inside #MAEK LIST"));
                }
                TokenKind::Annotation(tag) if tag == "#IHAZ" => {
                    return Err(self.error("a variable definition must be the first annotation in its block"));
                }
                TokenKind::Annotation(tag) if tag == "#MAEK HEAD" => {
                    return Err(self.error("the head may appear only immediately after #HAI and leading comments"));
                }
                TokenKind::Annotation(tag) => {
                    return Err(self.error(&format!("{} is not legal in this block", tag)));
                }
                TokenKind::Eof => return Err(self.error(&format!("expected {} before end of file", closing))),
            }
        }
        Ok(nodes)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BodyContext {
    Document,
    Paragraph,
    Item,
}

impl SyntaxAnalyzer for Parser {
    fn parse_lolcode(&mut self) -> Result<(), String> {
        self.root = Some(self.parse_document_node()?);
        Ok(())
    }
    fn parse_head(&mut self) -> Result<(), String> { self.parse_head_node().map(|_| ()) }
    fn parse_title(&mut self) -> Result<(), String> { self.parse_title_node().map(|_| ()) }
    fn parse_comment(&mut self) -> Result<(), String> { self.parse_comment_node().map(|_| ()) }
    fn parse_body(&mut self) -> Result<(), String> { self.parse_body_nodes("#KBYE", BodyContext::Document).map(|_| ()) }
    fn parse_paragraph(&mut self) -> Result<(), String> { self.parse_paragraph_node().map(|_| ()) }
    fn parse_inner_paragraph(&mut self) -> Result<(), String> { self.parse_body_nodes("#MKAY", BodyContext::Paragraph).map(|_| ()) }
    fn parse_inner_text(&mut self) -> Result<(), String> { self.parse_bold_node().map(|_| ()) }
    fn parse_variable_define(&mut self) -> Result<(), String> { self.parse_variable_definition_node().map(|_| ()) }
    fn parse_variable_use(&mut self) -> Result<(), String> { self.parse_variable_use_node().map(|_| ()) }
    fn parse_bold(&mut self) -> Result<(), String> { self.parse_bold_node().map(|_| ()) }
    fn parse_list(&mut self) -> Result<(), String> { self.parse_list_node().map(|_| ()) }
    fn parse_list_items(&mut self) -> Result<(), String> { self.parse_list_node().map(|_| ()) }
    fn parse_inner_list(&mut self) -> Result<(), String> { self.parse_body_nodes("#OIC", BodyContext::Item).map(|_| ()) }
    fn parse_link(&mut self) -> Result<(), String> { self.parse_link_node().map(|_| ()) }
    fn parse_newline(&mut self) -> Result<(), String> {
        self.expect_annotation("#NEWLINE")
    }
    fn parse_text(&mut self) -> Result<(), String> {
        match self.current().kind {
            TokenKind::Text(_) => { self.advance(); Ok(()) }
            _ => Err(self.error("expected text")),
        }
    }
}

/// Escapes generated text so the compiler always produces valid HTML even if
/// a future input contains an HTML-sensitive character.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Traverses the AST, resolves statically scoped variables, and emits HTML.
/// Each paragraph and list item introduces a nested scope; lookup proceeds
/// from the innermost active scope outward so shadowing behaves predictably.
fn render_nodes(
    nodes: &[Node],
    scopes: &mut Vec<HashMap<String, String>>,
    output: &mut String,
) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Document(children) => render_nodes(children, scopes, output)?,
            Node::Comment(text) => output.push_str(&format!("<!--{}-->", escape_html(text.trim()))),
            Node::Head(children) => {
                output.push_str("<head>");
                render_nodes(children, scopes, output)?;
                output.push_str("</head>");
            }
            Node::Title(text) => output.push_str(&format!("<title>{}</title>", escape_html(text.trim()))),
            Node::Paragraph(children) => {
                // Definitions inside a paragraph disappear when its closing
                // #MKAY is reached.
                scopes.push(HashMap::new());
                output.push_str("<p>");
                render_nodes(children, scopes, output)?;
                output.push_str("</p>");
                scopes.pop();
            }
            Node::Bold(children) => {
                output.push_str("<b>");
                render_nodes(children, scopes, output)?;
                output.push_str("</b>");
            }
            Node::List(children) => {
                output.push_str("<ul>");
                render_nodes(children, scopes, output)?;
                output.push_str("</ul>");
            }
            Node::Item(children) => {
                // Each item receives its own scope, preventing one item's
                // local definition from leaking into the next item.
                scopes.push(HashMap::new());
                output.push_str("<li>");
                render_nodes(children, scopes, output)?;
                output.push_str("</li>");
                scopes.pop();
            }
            Node::Newline => output.push_str("<br>"),
            Node::Link(address) => {
                let safe = escape_html(address.trim());
                output.push_str(&format!("<a href=\"{}\">{}</a>", safe, safe));
            }
            Node::Text(text) => output.push_str(&escape_html(text)),
            Node::VariableDefinition { name, value } => {
                let scope = scopes.last_mut().expect("at least one scope");
                scope.insert(name.clone(), value.clone());
            }
            Node::VariableUse(name) => {
                // Reverse iteration implements lexical/static lookup: the
                // nearest active declaration wins.
                let value = scopes
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(name))
                    .ok_or_else(|| format!("semantic error: variable '{}' is used before it is defined", name))?;
                output.push_str(&escape_html(value));
            }
        }
    }
    Ok(())
}

/// Runs all compiler phases in memory and returns HTML only after every phase
/// has succeeded.
fn compile_source(source: &str) -> Result<String, String> {
    let tokens = Scanner::new(source).all_tokens()?;
    let mut parser = Parser::new(tokens);
    SyntaxAnalyzer::parse_lolcode(&mut parser)?;
    let root = parser.root.ok_or_else(|| "syntax error: no document was produced".to_string())?;
    let mut output = String::from("<!DOCTYPE html>\n<html>");
    let mut scopes = vec![HashMap::new()];
    if let Node::Document(children) = root {
        render_nodes(&children, &mut scopes, &mut output)?;
    }
    output.push_str("</html>\n");
    Ok(output)
}

struct LolCompiler {
    scanner: Scanner,
    current: String,
    html: String,
}

impl LolCompiler {
    fn new() -> Self {
        Self { scanner: Scanner::new(""), current: String::new(), html: String::new() }
    }
}

impl Compiler for LolCompiler {
    fn compile(&mut self, source: &str) {
        match compile_source(source) {
            Ok(html) => self.html = html,
            Err(message) => {
                eprintln!("{}", message);
                std::process::exit(1);
            }
        }
    }

    fn next_token(&mut self) -> String {
        match self.scanner.next_token_result() {
            Ok(Token { kind: TokenKind::Annotation(value), .. }) => value,
            Ok(Token { kind: TokenKind::Text(value), .. }) => value,
            Ok(Token { kind: TokenKind::Eof, .. }) => String::new(),
            Err(message) => {
                eprintln!("{}", message);
                std::process::exit(1);
            }
        }
    }

    fn parse(&mut self) {
        let source: String = self.scanner.input.iter().collect();
        self.compile(&source);
    }

    fn current_token(&self) -> String { self.current.clone() }
    fn set_current_token(&mut self, tok: String) { self.current = tok; }
}

/// Returns the required output location beside the input with an .html suffix.
fn output_path(input: &Path) -> PathBuf {
    input.with_extension("html")
}

/// Opens a successful compilation in the operating system's default browser.
/// Tests set LOLCODE_NO_OPEN=1 so they can run without creating browser tabs.
fn open_browser(path: &Path) {
    if env::var("LOLCODE_NO_OPEN").ok().as_deref() == Some("1") {
        return;
    }
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(path).status();
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", ""]).arg(path).status();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let result = Command::new("xdg-open").arg(path).status();
    if let Err(error) = result {
        eprintln!("warning: HTML was created, but the browser could not be opened: {}", error);
    }
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 2 || !arguments[1].to_ascii_lowercase().ends_with(".lol") {
        eprintln!("usage: lolcompiler <input.lol>");
        std::process::exit(1);
    }

    let input = Path::new(&arguments[1]);
    let source = fs::read_to_string(input).unwrap_or_else(|error| {
        eprintln!("input error: could not read '{}': {}", input.display(), error);
        std::process::exit(1);
    });

    // Remove an older output before compilation. If this run fails, the
    // required observable result is therefore no HTML output file.
    let output = output_path(input);
    if output.exists() {
        fs::remove_file(&output).unwrap_or_else(|error| {
            eprintln!("output error: could not remove old '{}': {}", output.display(), error);
            std::process::exit(1);
        });
    }

    let mut compiler = LolCompiler::new();
    compiler.compile(&source);
    fs::write(&output, &compiler.html).unwrap_or_else(|error| {
        eprintln!("output error: could not write '{}': {}", output.display(), error);
        std::process::exit(1);
    });
    println!("Compiled '{}' -> '{}'", input.display(), output.display());
    open_browser(&output);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_all_annotations() {
        let source = r#"#HAI
#OBTW test document #TLDR
#MAEK HEAD #GIMMEH TITLE Kiara Test #OIC #MKAY
#MAEK PARAGRAF
#IHAZ lastname #ITIZ Student #MKAY
Hello #GIMMEH BOLD #GIMMEH VAR lastname #OIC #OIC #NEWLINE
#GIMMEH LINX www.towson.edu #OIC
#MAEK LIST #GIMMEH ITEM First #OIC #GIMMEH ITEM Second #OIC #MKAY
#MKAY
#KBYE"#;
        let html = compile_source(source).expect("valid program should compile");
        assert!(html.contains("<title>Kiara Test</title>"));
        assert!(html.contains("<b>Student</b>"));
        assert!(html.contains("<ul>"));
    }

    #[test]
    fn resolves_inner_and_outer_static_scopes() {
        let source = r#"#HAI
#IHAZ lastname #ITIZ Watson #MKAY
#MAEK PARAGRAF
#IHAZ lastname #ITIZ Student #MKAY
Inside #GIMMEH VAR lastname #OIC
#MKAY
Outside #GIMMEH VAR lastname #OIC
#KBYE"#;
        let html = compile_source(source).unwrap();
        assert!(html.contains("Inside Student"));
        assert!(html.contains("Outside Watson"));
    }

    #[test]
    fn rejects_unknown_annotation() {
        assert!(compile_source("#HAI #HEY no #KBYE").unwrap_err().contains("lexical error"));
    }

    #[test]
    fn rejects_item_outside_list() {
        assert!(compile_source("#HAI #GIMMEH ITEM no #OIC #KBYE").unwrap_err().contains("only inside"));
    }

    #[test]
    fn rejects_nested_paragraph() {
        let error = compile_source("#HAI #MAEK PARAGRAF #MAEK PARAGRAF no #MKAY #MKAY #KBYE").unwrap_err();
        assert!(error.contains("not legal in this block"));
    }

    #[test]
    fn rejects_undefined_variable() {
        let error = compile_source("#HAI #GIMMEH VAR missing #OIC #KBYE").unwrap_err();
        assert!(error.contains("semantic error"));
    }

    #[test]
    fn accepts_case_insensitive_annotations() {
        let html = compile_source("#hai hello #newline world #kbye").unwrap();
        assert!(html.contains("<br>"));
    }

    #[test]
    fn accepts_an_empty_document() {
        let html = compile_source("#HAI #KBYE").unwrap();
        assert_eq!(html, "<!DOCTYPE html>\n<html></html>\n");
    }

    #[test]
    fn accepts_comments_around_a_head_title() {
        let source = "#HAI #OBTW before #TLDR #MAEK HEAD #OBTW inside #TLDR \
                      #GIMMEH TITLE My Page #OIC #OBTW after #TLDR #MKAY #KBYE";
        let html = compile_source(source).unwrap();
        assert!(html.contains("<head><!--inside--><title>My Page</title><!--after--></head>"));
    }

    #[test]
    fn allows_a_variable_inside_bold_text() {
        let source = "#HAI #IHAZ word #ITIZ strong #MKAY \
                      #GIMMEH BOLD #GIMMEH VAR word #OIC #OIC #KBYE";
        let html = compile_source(source).unwrap();
        assert!(html.contains("<b>strong</b>"));
    }

    #[test]
    fn keeps_item_scopes_separate() {
        let source = "#HAI #IHAZ name #ITIZ outer #MKAY #MAEK LIST \
                      #GIMMEH ITEM #IHAZ name #ITIZ first #MKAY \
                      #GIMMEH VAR name #OIC #OIC \
                      #GIMMEH ITEM #GIMMEH VAR name #OIC #OIC #MKAY #KBYE";
        let html = compile_source(source).unwrap();
        assert!(html.contains("<li>first</li>"));
        assert!(html.contains("<li>outer</li>"));
    }

    #[test]
    fn escapes_html_sensitive_text() {
        let html = compile_source("#HAI A & B < C #KBYE").unwrap();
        assert!(html.contains("A &amp; B &lt; C"));
    }

    #[test]
    fn rejects_a_title_outside_the_head() {
        let error = compile_source("#HAI #GIMMEH TITLE Wrong place #OIC #KBYE").unwrap_err();
        assert!(error.contains("not legal in this block"));
    }

    #[test]
    fn rejects_an_empty_list() {
        let error = compile_source("#HAI #MAEK LIST #MKAY #KBYE").unwrap_err();
        assert!(error.contains("must begin immediately"));
    }

    #[test]
    fn rejects_a_second_title() {
        let source = "#HAI #MAEK HEAD #GIMMEH TITLE One #OIC \
                      #GIMMEH TITLE Two #OIC #MKAY #KBYE";
        assert!(compile_source(source).unwrap_err().contains("only one title"));
    }

    #[test]
    fn rejects_a_late_variable_definition() {
        let source = "#HAI #MAEK PARAGRAF text #IHAZ value #ITIZ late #MKAY #MKAY #KBYE";
        assert!(compile_source(source).unwrap_err().contains("first annotation"));
    }

    #[test]
    fn rejects_text_after_the_document() {
        assert!(compile_source("#HAI #KBYE extra").unwrap_err().contains("nothing may appear"));
    }

    #[test]
    fn rejects_missing_closing_annotations() {
        let error = compile_source("#HAI #MAEK PARAGRAF unfinished #KBYE").unwrap_err();
        assert!(error.contains("not legal in this block") || error.contains("before end of file"));
    }
}
