#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // System Structure Keywords
    System,
    Module,
    Domain,
    State,
    Transition,
    Goal,
    Reconcile,
    Migration,
    
    // Execution & Scheduling
    Start,
    Step,
    Slice,
    Emit,
    Send,
    On,
    When,
    Target,
    Strategy,
    Priority,

    // Scope & Authority
    Scope,
    Elevate,
    Capability,

    // Visibility & Composition
    Public,
    Private,
    Import,

    // Control Flow
    If,
    Else,
    Let,

    // Symbols & Operators
    LBrace,      // {
    RBrace,      // }
    LParen,      // (
    RParen,      // )
    Colon,       // :
    Equal,       // =
    EqualEqual,  // ==
    NotEqual,    // !=
    LessThan,    // <
    GreaterThan, // >
    LTEqual,     // <=
    GTEqual,     // >=
    Arrow,       // ->
    Plus,        // +
    Minus,       // -
    Asterisk,    // *
    Slash,       // /
    Modulo,      // %
    Dot,         // .
    Comma,       // ,

    // Literals and Identifiers
    Identifier(String),
    IntLiteral(i64),
    StringLiteral(String),

    EOF,
    Unknown(char),
}

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer { input, position: 0 }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.advance_char();
            } else {
                break;
            }
        }
    }
    
    fn read_identifier_or_keyword(&mut self) -> Token {
        let start = self.position;
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.advance_char();
            } else {
                break;
            }
        }
        let text = &self.input[start..self.position];
        match text {
            "system" => Token::System,
            "module" => Token::Module,
            "domain" => Token::Domain,
            "state" => Token::State,
            "transition" => Token::Transition,
            "goal" => Token::Goal,
            "reconcile" => Token::Reconcile,
            "migration" => Token::Migration,
            "start" => Token::Start,
            "step" => Token::Step,
            "slice" => Token::Slice,
            "emit" => Token::Emit,
            "send" => Token::Send,
            "on" => Token::On,
            "when" => Token::When,
            "target" => Token::Target,
            "strategy" => Token::Strategy,
            "priority" => Token::Priority,
            "scope" => Token::Scope,
            "elevate" => Token::Elevate,
            "capability" => Token::Capability,
            "public" => Token::Public,
            "private" => Token::Private,
            "import" => Token::Import,
            "if" => Token::If,
            "else" => Token::Else,
            "let" => Token::Let,
            _ => Token::Identifier(text.to_string()),
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.position;
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.advance_char();
            } else {
                break;
            }
        }
        let text = &self.input[start..self.position];
        Token::IntLiteral(text.parse().unwrap_or(0))
    }

    fn read_string(&mut self) -> Token {
        self.advance_char(); // skip opening quote
        let start = self.position;
        while let Some(c) = self.peek_char() {
            if c == '"' {
                break;
            }
            self.advance_char();
        }
        let text = &self.input[start..self.position];
        self.advance_char(); // skip closing quote
        Token::StringLiteral(text.to_string())
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let ch = match self.peek_char() {
            Some(c) => c,
            None => return Token::EOF,
        };

        if ch.is_alphabetic() || ch == '_' {
            return self.read_identifier_or_keyword();
        }

        if ch.is_ascii_digit() {
            return self.read_number();
        }

        match ch {
            '{' => { self.advance_char(); Token::LBrace }
            '}' => { self.advance_char(); Token::RBrace }
            '(' => { self.advance_char(); Token::LParen }
            ')' => { self.advance_char(); Token::RParen }
            ':' => { self.advance_char(); Token::Colon }
            '.' => { self.advance_char(); Token::Dot }
            ',' => { self.advance_char(); Token::Comma }
            '+' => { self.advance_char(); Token::Plus }
            '-' => {
                self.advance_char();
                if self.peek_char() == Some('>') {
                    self.advance_char();
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }
            '*' => { self.advance_char(); Token::Asterisk }
            '/' => { self.advance_char(); Token::Slash }
            '%' => { self.advance_char(); Token::Modulo }
            '=' => {
                self.advance_char();
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    Token::EqualEqual
                } else {
                    Token::Equal
                }
            }
            '!' => {
                self.advance_char();
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    Token::NotEqual
                } else {
                    Token::Unknown('!')
                }
            }
            '<' => {
                self.advance_char();
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    Token::LTEqual
                } else {
                    Token::LessThan
                }
            }
            '>' => {
                self.advance_char();
                if self.peek_char() == Some('=') {
                    self.advance_char();
                    Token::GTEqual
                } else {
                    Token::GreaterThan
                }
            }
            '"' => self.read_string(),
            _ => { self.advance_char(); Token::Unknown(ch) },
        }
    }
}

pub fn lex(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        if tok == Token::EOF {
            tokens.push(tok);
            break;
        }
        tokens.push(tok);
    }
    tokens
}
