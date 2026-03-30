#![allow(dead_code)]
//! Syntax highlighting via tree-sitter.
use tree_sitter::{Language, Parser, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Default,
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Type,
    Variable,
    Operator,
    Punctuation,
    Constant,
    Attribute,
}

#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub kind: TokenKind,
}

pub struct Highlighter {
    parser: Parser,
    language: Language,
    tree: Option<Tree>,
    lang_name: String,
}

impl Highlighter {
    fn new(language: Language, lang_name: &str) -> Option<Self> {
        let mut parser = Parser::new();
        parser.set_language(language).ok()?;
        Some(Self {
            parser,
            language,
            tree: None,
            lang_name: lang_name.to_string(),
        })
    }

    pub fn for_rust() -> Option<Self> {
        Self::new(tree_sitter_rust::language(), "rust")
    }

    pub fn for_python() -> Option<Self> {
        Self::new(tree_sitter_python::language(), "python")
    }

    pub fn for_javascript() -> Option<Self> {
        Self::new(tree_sitter_javascript::language(), "javascript")
    }

    pub fn for_json() -> Option<Self> {
        Self::new(tree_sitter_json::language(), "json")
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Self::for_rust(),
            "py" => Self::for_python(),
            "js" | "jsx" | "mjs" => Self::for_javascript(),
            "json" => Self::for_json(),
            _ => None,
        }
    }

    /// Parse source and return highlight spans for all tokens.
    pub fn highlight(&mut self, source: &[u8]) -> Vec<HighlightSpan> {
        let tree = match self.parser.parse(source, None) {
            Some(t) => t,
            None => return Vec::new(),
        };
        self.tree = Some(tree.clone());
        self.collect_spans(&tree, source)
    }

    /// Re-parse (incremental in future, full for now) and return spans.
    pub fn update(&mut self, source: &[u8]) -> Vec<HighlightSpan> {
        self.highlight(source)
    }

    fn collect_spans(&self, tree: &Tree, source: &[u8]) -> Vec<HighlightSpan> {
        let mut spans = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        let lang = self.lang_name.as_str();

        // Walk the tree iteratively
        let mut visited_children = false;
        loop {
            let node = cursor.node();

            if !visited_children {
                let kind = node.kind();
                let start = node.start_byte();
                let end = node.end_byte();

                // Only emit leaf nodes (no children) or named non-leaf keywords
                if node.child_count() == 0 {
                    let token_kind = classify_node(kind, source, start, end, lang);
                    if token_kind != TokenKind::Default {
                        spans.push(HighlightSpan { start_byte: start, end_byte: end, kind: token_kind });
                    }
                }

                if cursor.goto_first_child() {
                    continue;
                }
            }

            if cursor.goto_next_sibling() {
                visited_children = false;
                continue;
            }

            if !cursor.goto_parent() {
                break;
            }
            visited_children = true;
        }

        spans
    }
}

fn classify_node(kind: &str, source: &[u8], start: usize, end: usize, lang: &str) -> TokenKind {
    match kind {
        // Comments
        "line_comment" | "block_comment" | "comment" => TokenKind::Comment,

        // Strings
        "string_literal" | "string" | "interpreted_string_literal"
        | "raw_string_literal" | "string_content" => TokenKind::String,

        // Numbers
        "integer_literal" | "float_literal" | "number" | "integer" => TokenKind::Number,

        // Operators
        "==" | "!=" | "<=" | ">=" | "&&" | "||"
        | "+=" | "-=" | "*=" | "/=" | "%=" | "=>" | "->" | "::" => TokenKind::Operator,

        // Punctuation
        "(" | ")" | "[" | "]" | "{" | "}" | ";" | "," | "." | ":" => TokenKind::Punctuation,

        // Type identifiers (Rust)
        "primitive_type" => TokenKind::Type,

        // Attributes
        "attribute_item" | "meta_item" | "decorator" => TokenKind::Attribute,

        // Function item
        "function_item" => TokenKind::Function,

        // Rust-specific keyword node kinds
        "fn" | "let" | "mut" | "pub" | "use" | "mod" | "struct" | "enum" | "impl"
        | "trait" | "return" | "if" | "else" | "match" | "for" | "while" | "loop"
        | "break" | "continue" | "const" | "static" | "type" | "where" | "async"
        | "await" | "move" | "ref" | "in" | "as" | "dyn" | "unsafe" | "extern"
        | "crate" | "super" | "self" | "Self" => TokenKind::Keyword,

        // Shared keyword node kinds (Python/JS overlap handled by lang check)
        "import" | "from" | "class" | "try" | "except" | "finally" | "with"
        | "yield" | "raise" | "pass" | "def" | "lambda" | "del" | "global"
        | "nonlocal" | "assert" | "not" | "and" | "or" | "is"
        | "None" | "True" | "False"
        | "var" | "function" | "typeof" | "instanceof" | "void"
        | "throw" | "catch" | "switch" | "case" | "default" | "this"
        | "extends" | "export" | "of" | "do"
        | "debugger" | "null" | "undefined" | "new" | "delete" => TokenKind::Keyword,

        // Identifiers: check text against keyword list for the language
        "identifier" | "type_identifier" => {
            if let Ok(text) = std::str::from_utf8(&source[start..end]) {
                match lang {
                    "rust" if is_rust_keyword(text) => TokenKind::Keyword,
                    "rust" => TokenKind::Default,
                    "python" if is_python_keyword(text) => TokenKind::Keyword,
                    "python" => TokenKind::Default,
                    "javascript" if is_js_keyword(text) => TokenKind::Keyword,
                    _ => TokenKind::Default,
                }
            } else {
                TokenKind::Default
            }
        }

        _ => TokenKind::Default,
    }
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "fn" | "let" | "mut" | "pub" | "use" | "mod" | "struct" | "enum" | "impl"
        | "trait" | "return" | "if" | "else" | "match" | "for" | "while" | "loop"
        | "break" | "continue" | "const" | "static" | "type" | "where" | "async"
        | "await" | "move" | "ref" | "in" | "as" | "dyn" | "unsafe" | "extern"
        | "crate" | "super" | "self" | "Self" | "true" | "false" | "None" | "Some"
        | "Ok" | "Err"
    )
}

fn is_python_keyword(s: &str) -> bool {
    matches!(
        s,
        "def" | "class" | "import" | "from" | "with" | "pass" | "return" | "raise"
        | "try" | "except" | "finally" | "yield" | "lambda" | "del" | "global"
        | "nonlocal" | "assert" | "not" | "and" | "or" | "is" | "in" | "if"
        | "else" | "elif" | "for" | "while" | "break" | "continue" | "None"
        | "True" | "False" | "async" | "await"
    )
}

fn is_js_keyword(s: &str) -> bool {
    matches!(
        s,
        "var" | "let" | "const" | "function" | "return" | "if" | "else" | "for"
        | "while" | "do" | "break" | "continue" | "switch" | "case" | "default"
        | "try" | "catch" | "finally" | "throw" | "new" | "delete" | "typeof"
        | "instanceof" | "void" | "class" | "extends" | "super" | "import"
        | "export" | "from" | "async" | "await" | "yield" | "this" | "of"
        | "in" | "null" | "undefined" | "true" | "false" | "debugger" | "with"
    )
}
