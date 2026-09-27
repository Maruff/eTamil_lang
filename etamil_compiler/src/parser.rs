// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
use crate::lexer::{Spanned, Token};
use rust_decimal::Decimal;
use std::iter::Peekable;
use std::slice::Iter;

/// A parse error, carrying the position of the token that caused it.
///
/// The parser used to `panic!` with `Expected Semicolon` and no location,
/// which was the biggest usability gap in the language for anyone learning
/// it. Messages are bilingual, as the lexer's already were.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    /// What the parser was looking for.
    pub expected: String,
    /// The source text actually found, empty at the end of input.
    pub found: String,
    /// True when `found` is a word the parser cannot accept as a name.
    ///
    /// The language reserves 203 words, and they include common nouns — சொல்,
    /// உரை, தரவு, தலைப்பு, இடம், வரிசை, வரம்பு, முகவரி, பதில், எண். Using one
    /// as a variable or a parameter is an ordinary mistake, and without this
    /// the message describes the symptom rather than the cause.
    pub reserved: bool,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.found.is_empty() {
            write!(
                f,
                "வரி {}, நெடுவரிசை {}: {} எதிர்பார்க்கப்பட்டது, உள்ளீடு முடிந்தது  \
                 (line {}, column {}: expected {}, but the input ended)",
                self.line, self.column, self.expected, self.line, self.column, self.expected
            )
        } else {
            write!(
                f,
                "வரி {}, நெடுவரிசை {}: {} எதிர்பார்க்கப்பட்டது, '{}' கிடைத்தது  \
                 (line {}, column {}: expected {}, found '{}')",
                self.line,
                self.column,
                self.expected,
                self.found,
                self.line,
                self.column,
                self.expected,
                self.found
            )?;
            if self.reserved {
                write!(
                    f,
                    "\n  '{}' ஒரு ஒதுக்கப்பட்ட சொல்; அதைப் பெயராகப் பயன்படுத்த முடியாது  \
                     ('{}' is a reserved word and cannot be used as a name)",
                    self.found, self.found
                )?;
            }
            Ok(())
        }
    }
}

// --- Abstract Syntax Tree (AST) Nodes ---

/// Where something was written, carried on the AST nodes that can be rejected
/// after parsing, so a type error points at a place rather than only naming a
/// variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// A type written in the source: `எண் வருவாய் = 100000;`
///
/// Type keywords used to be parsed and thrown away, so `சொல் x = 5;` was
/// accepted. Keeping the declaration is what lets the checker hold an
/// assignment to it.
///
/// There is one numeric type. `எண்` and `பின்னம்` both mean Number, because
/// every value in the language is already a fixed-point decimal — a separate
/// integer type would be a second decision, not a consequence of this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredType {
    Number,
    Text,
    Boolean,
    Array,
    Record,
    Date,
    /// `செயல்` in a type position: a function value, `செயல் மாற்று`.
    Function,
    /// A வடிவம் the program declared, by name: `கடன் க = கடன்{…};`.
    Shape(String),
    /// No constraint: used where a type keyword exists that the checker has
    /// nothing to say about yet.
    Any,
}

/// One parameter of a `செயல்`, with the type it was declared as if it was.
///
/// `at` is where the name was written, so a call that passes the wrong thing
/// can be pointed at the parameter it disagrees with.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub declared: Option<DeclaredType>,
    /// Declared `நிலை`: the body may read it and never assign to it.
    pub immutable: bool,
    pub at: Position,
}

/// One field a வடிவம் declares: `எண் அசல்`.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeField {
    pub name: String,
    pub declared: Option<DeclaredType>,
    pub at: Position,
}

/// A செயல் written inside a வடிவம். Its first parameter is the record it was
/// called on when that parameter is named இது — see `is_self` — and it is an
/// associated function, called as `கடன்.புதிது(…)`, when it is not.
#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub params: Vec<Param>,
    pub returns: Option<DeclaredType>,
    pub body: Vec<Stmt>,
    pub at: Position,
}

impl Method {
    /// Is it called on a record, `க.வட்டி(2)`, rather than on the shape?
    pub fn takes_self(&self) -> bool {
        self.params
            .first()
            .is_some_and(|param| is_self(&param.name))
    }
}

/// `இது`, in any of its three spellings: the record a method was called on.
///
/// A name rather than a keyword, as Rust's `self` is not: it is only special as
/// the first parameter of a method, and anywhere else it is a name like any
/// other. The name of the function a method compiles to — `கடன்.வட்டி` — has a
/// dot in it, which no source identifier can, so it cannot collide either.
pub fn is_self(name: &str) -> bool {
    matches!(name, "இது" | "iqu" | "_self")
}

impl DeclaredType {
    /// The keyword an author would have written, for error messages.
    pub fn name(&self) -> String {
        let fixed = match self {
            DeclaredType::Number => "எண் (eN, a number)",
            DeclaredType::Text => "சொல் (col, a string)",
            DeclaredType::Boolean => "ஈர்ம (Irma, a boolean)",
            DeclaredType::Array => "அணி (aNi, an array)",
            DeclaredType::Record => "பொருள் (poruL, a record)",
            DeclaredType::Date => "தேதி (qEqi, a date)",
            DeclaredType::Function => "செயல் (ceyal, a function)",
            DeclaredType::Shape(shape) => return format!("{} (a வடிவம், shape)", shape),
            DeclaredType::Any => "any type",
        };
        fixed.to_string()
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(Decimal),
    String(String),
    Boolean(bool),
    Null,
    Variable(String),
    BinaryOp {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Comparison {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Concat {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    // maRRum / allaqu — both operands are evaluated (no short-circuiting)
    Logical {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    // illY
    Not(Box<Expr>),
    // name(arg, ...)
    Call {
        name: String,
        args: Vec<Expr>,
    },
    // அணி — a column: [a, b, c]
    ArrayLiteral(Vec<Expr>),
    // பொருள் — a row: {peyar: "ravi", vayaqu: 20}
    RecordLiteral(Vec<(String, Expr)>),
    // a[i]
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    // r.peyar
    Field {
        base: Box<Expr>,
        name: String,
        at: Position,
    },
    // r.vatti(2) — a method of r's shape, or a function r holds in a field.
    // `கடன்.புதிது(…)` with a shape's name for r is an associated function.
    MethodCall {
        receiver: Box<Expr>,
        name: String,
        args: Vec<Expr>,
        at: Position,
    },
    // கடன்{அசல்: 100000, வீதம்: 9%} — a record made as a declared shape.
    // `..பழையது` last takes every field not written from another கடன்.
    ShapeLiteral {
        shape: String,
        fields: Vec<(String, Expr)>,
        base: Option<Box<Expr>>,
        at: Position,
    },
    // expr? — unwrap a சரி, or return the தவறு to the caller
    Try(Box<Expr>),
    // செயல்(x) { திரும்பு x * 2; } — a function written where a value goes.
    // It carries the enclosing function's locals it uses, by value.
    Lambda {
        params: Vec<Param>,
        returns: Option<DeclaredType>,
        body: Vec<Stmt>,
        at: Position,
    },
    // f(1)(2), விதிகள்[0](தொகை) — call whatever an expression produced
    CallValue {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Stmt {
    Assign {
        name: String,
        value: Expr,
        /// The type the author wrote, if any: `எண் வருவாய் = 100000;`
        declared: Option<DeclaredType>,
        /// Where the name was written, for the checker to point at.
        at: Position,
        /// `நிலை பெயர் = …;` — bound once, never assigned again, and no part
        /// of it changed. Values are copied rather than shared, so holding
        /// the name still is enough to hold the whole value still.
        immutable: bool,
    },
    // ceyal name(params) returns { body }
    FunctionDef {
        name: String,
        params: Vec<Param>,
        /// The type the function promises to return, if it said.
        returns: Option<DeclaredType>,
        body: Vec<Stmt>,
        /// Where the function's name was written, so a திரும்பு that breaks
        /// the declared return type has somewhere to point.
        at: Position,
    },
    // வடிவம் கடன் { எண் அசல், எண் வீதம், செயல் வட்டி(இது) { … } }
    ShapeDef {
        name: String,
        fields: Vec<ShapeField>,
        methods: Vec<Method>,
        at: Position,
    },
    // qirumpu value;
    Return(Option<Expr>),
    // a[i] = value;  — the base must be a plain variable for now
    SetIndex {
        name: String,
        index: Expr,
        value: Expr,
        at: Position,
    },
    // r.peyar = value;
    SetField {
        name: String,
        field: String,
        value: Expr,
        at: Position,
    },
    // A bare expression evaluated for its effect, e.g. a call statement.
    Expression(Expr),
    // iRakku "path.qmz"; — resolved before compilation, see module.rs
    Import(String),
    Print(Expr),
    Input(Expr),
    // (cond) eZil { then } iZREl { else }
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    // (cond) cuRRu { body }
    Loop {
        condition: Expr,
        body: Vec<Stmt>,
    },
    // ovvoru item il collection { body }
    ForEach {
        var: String,
        collection: Expr,
        body: Vec<Stmt>,
    },
    // File I/O Operations
    FileOpen {
        filename: Expr,
        mode: String, // "read", "write", "append"
    },
    FileClose {
        filename: Expr,
    },
    FileWrite {
        filename: Expr,
        data: Expr,
    },
    FileRead {
        filename: Expr,
        variable: String,
    },
    ReadCSV {
        filename: Expr,
        variable: String,
    },
    WriteCSV {
        filename: Expr,
        data: Expr,
    },
    // Database Operations
    DBConnect {
        db_type: String,
        connection_string: Expr,
        /// The name this connection is known by. Defaults to the driver name,
        /// which is how a program that names nothing keeps working.
        handle: Option<String>,
    },
    DBDisconnect {
        db_type: String,
    },
    // தளம்_வினா "sql", [params], result;
    DBQuery {
        query: Expr,
        params: Expr,
        result_var: String,
        /// Which connection to ask. `None` means the only open one.
        handle: Option<String>,
    },
    // தளம்_செய் "sql", [params];
    DBExecute {
        command: Expr,
        params: Expr,
        /// Which connection to run this on. `None` means the only open one.
        handle: Option<String>,
    },
    DBInsert {
        table: String,
        data: Expr,
    },
    DBUpdate {
        table: String,
        data: Expr,
        condition: Option<Expr>,
    },
    DBDelete {
        table: String,
        condition: Expr,
    },
    CreateTable {
        table: String,
        schema: Expr,
    },
    Select {
        columns: Vec<String>,
        from_table: String,
        where_clause: Option<Expr>,
    },
    // REST API Operations
    DefineRoute {
        method: String,     // GET, POST, PUT, DELETE, etc.
        path: Expr,         // "/api/users"
        handler: Vec<Stmt>, // Handler code block
    },
    StartServer {
        host: Expr,
        port: Expr,
    },
    // இடைவெளி 60 { … } — run the block every 60 seconds. Lifted out of the
    // program at startup like வழி, and needs a server for the same reason.
    Schedule {
        seconds: Expr,
        body: Vec<Stmt>,
    },
    StopServer,
    // பதில் 200, உடல்;  or with headers as an ordinary record:
    // பதில் 200, உடல், {"Content-Type": "text/html"};
    SendResponse {
        status_code: Expr,
        body: Expr,
        headers: Option<Expr>,
    },
    SendJSON {
        data: Expr,
        status_code: Option<Expr>,
    },
    GetRequestBody {
        variable: String,
    },
    GetRequestParam {
        param_name: Expr,
        variable: String,
    },
    GetHeader {
        header_name: Expr,
        variable: String,
    },
    SetHeader {
        header_name: Expr,
        value: Expr,
    },
}

// --- Parser Implementation ---

pub struct Parser<'a> {
    tokens: Peekable<Iter<'a, Spanned>>,
    /// Where the last consumed token was, so an unexpected end of input can
    /// still be reported somewhere the author recognizes.
    last: (usize, usize),
    /// True while parsing an expression a `{` block follows — the collection
    /// of a ஒவ்வொரு, the seconds of an இடைவெளி, the path of a வழி — where
    /// `பெயர் {` must mean the block, not a shaped literal. Rust has the same
    /// rule for `for` and `if`, for the same reason. Brackets lift it.
    restrict: bool,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Iter<'a, Spanned>) -> Self {
        Parser {
            tokens: tokens.peekable(),
            last: (1, 1),
            restrict: false,
        }
    }

    /// Entry point: parse the whole token stream into statements.
    pub fn parse(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();
        while self.tokens.peek().is_some() {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    // --- Position and error helpers ---------------------------------------

    /// Look at the next token without consuming it.
    ///
    /// `Iter::peek` hands back a `&&Spanned`; copying the inner reference out
    /// frees the result from the borrow on `self`, so an error can be built
    /// from it in the same expression.
    fn peek_spanned(&mut self) -> Option<&'a Spanned> {
        self.tokens.peek().copied()
    }

    fn peek_token(&mut self) -> Option<&'a Token> {
        self.peek_spanned().map(|spanned| &spanned.token)
    }

    /// The token after the next one. The iterator is a cheap copy of a slice
    /// cursor, so looking two ahead costs nothing and consumes nothing.
    fn peek_after(&self) -> Option<&'a Token> {
        let mut ahead = self.tokens.clone();
        ahead.next();
        ahead.next().map(|spanned| &spanned.token)
    }

    fn advance(&mut self) -> Option<&'a Spanned> {
        let spanned = self.tokens.next();
        if let Some(spanned) = spanned {
            self.last = (spanned.line, spanned.column);
        }
        spanned
    }

    /// Consume the next token, or report what was wanted instead.
    fn take(&mut self, expected: &str) -> Result<&'a Spanned, ParseError> {
        match self.advance() {
            Some(spanned) => Ok(spanned),
            None => Err(self.at_end(expected)),
        }
    }

    fn mismatch(&self, spanned: &Spanned, expected: &str) -> ParseError {
        ParseError {
            line: spanned.line,
            column: spanned.column,
            expected: expected.to_string(),
            found: spanned.text.clone(),
            // Only where a name was actually wanted. is_identifier_like is the
            // parser's own answer to "may this word be a name?", so the test
            // cannot drift from what the parser accepts — but attaching it to
            // every keyword-shaped error told someone who forgot a semicolon
            // that அச்சு cannot be a variable, which is true and useless.
            reserved: expected.contains("name") && !Self::is_identifier_like(&spanned.token),
        }
    }

    fn at_end(&self, expected: &str) -> ParseError {
        ParseError {
            line: self.last.0,
            column: self.last.1,
            expected: expected.to_string(),
            found: String::new(),
            reserved: false,
        }
    }

    /// A readable name for a token the parser was expecting. Punctuation reads
    /// the same in either language; a keyword shows both spellings.
    fn describe(token: &Token) -> String {
        match token {
            Token::Semicolon => "';'",
            Token::Comma => "','",
            Token::Colon => "':'",
            Token::Assign => "'='",
            Token::LParen => "'('",
            Token::RParen => "')'",
            Token::LBrace => "'{'",
            Token::RBrace => "'}'",
            Token::LBracket => "'['",
            Token::RBracket => "']'",
            Token::In => "இல் (il)",
            Token::From => "இதனில் (iqaZil)",
            Token::Where => "விதி (viqi)",
            Token::If => "எனில் (eZil)",
            Token::Loop => "சுற்று (cuRRu)",
            Token::Else => "இன்றேல் (iZREl)",
            other => return format!("{:?}", other),
        }
        .to_string()
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        match self.peek_spanned() {
            Some(spanned) if spanned.token == expected => {
                self.advance();
                Ok(())
            }
            Some(spanned) => Err(self.mismatch(spanned, &Self::describe(&expected))),
            None => Err(self.at_end(&Self::describe(&expected))),
        }
    }

    fn matches(&mut self, expected: Token) -> bool {
        match self.peek_spanned() {
            Some(spanned) if spanned.token == expected => {
                self.advance();
                true
            }
            _ => false,
        }
    }

    // --- Names -------------------------------------------------------------

    /// A name as the author wrote it.
    ///
    /// This is what keeping the source text was for. The spelling used to be
    /// discarded and only the token kept, so `வங்கி = 5` created a variable
    /// called `Bank`: a Tamil author's chosen name was silently anglicised,
    /// printing such a record emitted English field names into Tamil output,
    /// and looking a field up by string needed the token name rather than the
    /// written one.
    ///
    /// The documented consequence: `{வரி: 1}` and `{vari: 1}` are now
    /// *different* fields. A field name is data, not a language construct.
    fn name_of(&self, spanned: &Spanned) -> String {
        match &spanned.token {
            // A quoted key carries its parsed contents, not its quotes.
            Token::String(text) => text.clone(),
            _ => spanned.text.clone(),
        }
    }

    /// Consume a token that must be usable as a name.
    fn take_name(&mut self, expected: &str) -> Result<String, ParseError> {
        let spanned = self.take(expected)?;
        let usable = matches!(spanned.token, Token::String(_))
            || (Self::is_identifier_like(&spanned.token) && !Self::is_type_token(&spanned.token));

        if !usable {
            return Err(self.mismatch(spanned, expected));
        }
        Ok(self.name_of(spanned))
    }

    /// The canonical English name of a keyword.
    ///
    /// Kept for the two places where a keyword names something the host must
    /// recognize rather than something the author invented: a database type,
    /// which `db::open` matches on, and an HTTP method, which the router
    /// matches on. Those are not the author's names, so they do not follow the
    /// author's spelling.
    /// A trailing `, name` naming which connection a statement means.
    ///
    /// Absent for every program written before handles existed, which is why
    /// it is optional rather than a new required slot.
    fn optional_handle(&mut self, expected: &str) -> Result<Option<String>, ParseError> {
        if !self.matches(Token::Comma) {
            return Ok(None);
        }
        let spanned = self.take(expected)?;
        Ok(Some(Self::token_name(&spanned.token)))
    }

    fn token_name(token: &Token) -> String {
        match token {
            Token::Identifier(name) => name.clone(),
            Token::SQL => "SQL".to_string(),
            Token::NoSQL => "NoSQL".to_string(),
            Token::SQLite => "SQLite".to_string(),
            Token::MySQL => "MySQL".to_string(),
            Token::PostgreSQL => "PostgreSQL".to_string(),
            Token::MongoDB => "MongoDB".to_string(),
            Token::Redis => "Redis".to_string(),
            Token::JSONdb => "JSONdb".to_string(),
            Token::HttpGet => "GET".to_string(),
            Token::HttpPost => "POST".to_string(),
            Token::HttpPut => "PUT".to_string(),
            Token::HttpDelete => "DELETE".to_string(),
            Token::HttpPatch => "PATCH".to_string(),
            Token::HttpOptions => "OPTIONS".to_string(),
            Token::HttpHead => "HEAD".to_string(),
            other => format!("{:?}", other),
        }
    }

    // --- Statements --------------------------------------------------------

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        // A statement is never the head of a block, even inside one.
        self.unrestricted(|parser| parser.parse_statement_inner())
    }

    /// Run `parse` with shaped literals allowed again, as they are inside any
    /// bracket, and put the restriction back afterwards.
    fn unrestricted<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let outer = std::mem::replace(&mut self.restrict, false);
        let parsed = parse(self);
        self.restrict = outer;
        parsed
    }

    /// An expression a `{` block follows, where `பெயர் {` is the block.
    fn parse_head(&mut self) -> Result<Expr, ParseError> {
        let outer = std::mem::replace(&mut self.restrict, true);
        let parsed = self.parse_expression();
        self.restrict = outer;
        parsed
    }

    /// A type written before a name, if one is: a type keyword, `செயல்`, or
    /// the name of a வடிவம். A shape's name is an ordinary identifier, so it is
    /// a type only where a second name follows it — `கடன் க` — which nothing
    /// else in the language allows.
    fn parse_type_prefix(&mut self) -> Option<DeclaredType> {
        match self.peek_token() {
            Some(token) if Self::is_type_token(token) => {
                let token = self.advance().expect("peeked");
                Some(Self::type_of(&token.token))
            }
            Some(Token::Function) if self.peek_after().is_some_and(Self::is_plain_name) => {
                self.advance();
                Some(DeclaredType::Function)
            }
            Some(token)
                if Self::is_plain_name(token)
                    && self.peek_after().is_some_and(Self::is_plain_name) =>
            {
                let spanned = self.advance().expect("peeked");
                Some(DeclaredType::Shape(self.name_of(spanned)))
            }
            _ => None,
        }
    }

    /// A token that can be a name and is not a type keyword.
    fn is_plain_name(token: &Token) -> bool {
        Self::is_identifier_like(token) && !Self::is_type_token(token)
    }

    /// Inside a வடிவம், is `செயல் பெயர்` a field's type and name — followed by
    /// `,` or `}` — rather than the start of a method?
    fn function_typed_field(&self) -> bool {
        let mut ahead = self.tokens.clone();
        ahead.next();
        let named = ahead
            .next()
            .is_some_and(|spanned| Self::is_plain_name(&spanned.token));
        named
            && matches!(
                ahead.next().map(|spanned| &spanned.token),
                Some(Token::Comma) | Some(Token::RBrace)
            )
    }

    /// Is this `வடிவம் பெயர் {`, rather than வடிவம் used as a name?
    fn starts_shape(&mut self) -> bool {
        self.peek_token().is_some_and(Self::is_plain_name)
            && self.peek_after() == Some(&Token::LBrace)
    }

    /// The rest of `வடிவம் பெயர் { … }`, the `வடிவம்` already consumed.
    ///
    /// Fields and methods may come in any order. A field is `[வகை] பெயர்`,
    /// separated by commas; a method is a `செயல்` exactly as it would be
    /// written outside, and needs no separator after its closing brace.
    fn parse_shape(&mut self) -> Result<Stmt, ParseError> {
        let spanned = self.take("a shape name")?;
        let name = self.name_of(spanned);
        let at = Position {
            line: spanned.line,
            column: spanned.column,
        };
        self.expect(Token::LBrace)?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();
        loop {
            if self.matches(Token::RBrace) {
                break;
            }
            if self.peek_token().is_none() {
                return Err(self.at_end("'}'"));
            }

            // `செயல் பெயர்(` is a method. `செயல் பெயர்,` is a field that holds a
            // function, which the type prefix below reads.
            if self.peek_token() == Some(&Token::Function) && !self.function_typed_field() {
                self.advance();
                let at = match self.peek_spanned() {
                    Some(spanned) => Position {
                        line: spanned.line,
                        column: spanned.column,
                    },
                    None => at,
                };
                let name = self.take_name("a method name")?;
                let (params, returns) = self.parse_signature()?;
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                methods.push(Method {
                    name,
                    params,
                    returns,
                    body,
                    at,
                });
                continue;
            }

            let declared = self.parse_type_prefix();
            let spanned = self.take("a field name")?;
            if !Self::is_plain_name(&spanned.token) {
                return Err(self.mismatch(spanned, "a field name"));
            }
            fields.push(ShapeField {
                name: self.name_of(spanned),
                declared,
                at: Position {
                    line: spanned.line,
                    column: spanned.column,
                },
            });
            if !self.matches(Token::Comma) && self.peek_token() != Some(&Token::RBrace) {
                // A method may follow a field directly; anything else is a
                // missing comma.
                if self.peek_token() != Some(&Token::Function) {
                    let expected = "',' or '}' after a field";
                    return Err(match self.peek_spanned() {
                        Some(spanned) => self.mismatch(spanned, expected),
                        None => self.at_end(expected),
                    });
                }
            }
        }

        Ok(Stmt::ShapeDef {
            name,
            fields,
            methods,
            at,
        })
    }

    fn parse_statement_inner(&mut self) -> Result<Stmt, ParseError> {
        let first = self.take("a statement")?;

        if first.token == Token::Shape && self.starts_shape() {
            return self.parse_shape();
        }

        // `நிலை பெயர் = …;` — an immutable binding. Only when a name or a type
        // follows: `நிலை = …;` assigns to a variable called நிலை, which two
        // examples do, and `{நிலை: …}` is a field. A hard keyword would have
        // broken both for no gain.
        if first.token == Token::Const && self.starts_fixed_binding() {
            return self.parse_fixed_binding();
        }

        // An optional type declaration: eN, piZZam, col and the rest. The
        // declared type is kept so the checker can hold assignments to it.
        let (declared, current) = if Self::is_type_token(&first.token) {
            let name = self.take("a name after the type")?;
            (Some(Self::type_of(&first.token)), name)
        } else if Self::is_plain_name(&first.token)
            && self.peek_token().is_some_and(Self::is_plain_name)
        {
            // `கடன் க = …;` — a name followed by a name: the first is a shape.
            let name = self.take("a name after the shape")?;
            (Some(DeclaredType::Shape(self.name_of(first))), name)
        } else {
            (None, first)
        };

        if Self::is_identifier_like(&current.token) && !Self::is_type_token(&current.token) {
            let name = self.name_of(current);
            let at = Position {
                line: current.line,
                column: current.column,
            };

            // A call used as a statement, e.g. `paqivu_ceyal(x);`
            if self.peek_token() == Some(&Token::LParen) {
                let call = self.finish_name_or_call(name)?;
                let call = self.parse_postfix(call)?;
                self.expect(Token::Semicolon)?;
                return Ok(Stmt::Expression(call));
            }

            // a[i] = value;
            if self.matches(Token::LBracket) {
                let index = self.parse_expression()?;
                self.expect(Token::RBracket)?;
                self.expect(Token::Assign)?;
                let value = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                return Ok(Stmt::SetIndex {
                    name,
                    index,
                    value,
                    at,
                });
            }

            // r.field = value;  or a method called for its effect: r.m(x);
            if self.matches(Token::Dot) {
                let field_at = self.peek_spanned().map(|spanned| Position {
                    line: spanned.line,
                    column: spanned.column,
                });
                let field = self.take_name("a field name")?;
                if self.matches(Token::LParen) {
                    let args = self.parse_arguments()?;
                    let call = Expr::MethodCall {
                        receiver: Box::new(Expr::Variable(name)),
                        name: field,
                        args,
                        at: field_at.unwrap_or(at),
                    };
                    let call = self.parse_postfix(call)?;
                    self.expect(Token::Semicolon)?;
                    return Ok(Stmt::Expression(call));
                }
                self.expect(Token::Assign)?;
                let value = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                return Ok(Stmt::SetField {
                    name,
                    field,
                    value,
                    at,
                });
            }

            // A declaration with no initializer.
            if self.matches(Token::Semicolon) {
                return Ok(Stmt::Assign {
                    name,
                    value: Expr::Number(Decimal::ZERO),
                    declared,
                    at,
                    immutable: false,
                });
            }

            self.expect(Token::Assign)?;
            let value = self.parse_expression()?;
            self.expect(Token::Semicolon)?;
            return Ok(Stmt::Assign {
                name,
                value,
                declared,
                at,
                immutable: false,
            });
        }

        match &current.token {
            Token::Function => {
                let at = match self.peek_spanned() {
                    Some(spanned) => Position {
                        line: spanned.line,
                        column: spanned.column,
                    },
                    None => Position {
                        line: self.last.0,
                        column: self.last.1,
                    },
                };
                let name = self.take_name("a function name")?;

                // `செயல் மாற்று = இரட்டி;` — a variable declared to hold a
                // function, the same shape as `எண் தொகை = 5;`.
                if self.matches(Token::Assign) {
                    let value = self.parse_expression()?;
                    self.expect(Token::Semicolon)?;
                    return Ok(Stmt::Assign {
                        name,
                        value,
                        declared: Some(DeclaredType::Function),
                        at,
                        immutable: false,
                    });
                }

                let (params, returns) = self.parse_signature()?;
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                Ok(Stmt::FunctionDef {
                    name,
                    params,
                    returns,
                    body,
                    at,
                })
            }
            Token::Import => {
                let path = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                match path {
                    Expr::String(path) => Ok(Stmt::Import(path)),
                    _ => Err(self.at_end("a quoted file path after இறக்கு (iRakku)")),
                }
            }
            Token::ForEach => {
                let var = self.take_name("a loop variable")?;
                self.expect(Token::In)?;
                let collection = self.parse_head()?;
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                Ok(Stmt::ForEach {
                    var,
                    collection,
                    body,
                })
            }
            Token::Return => {
                if self.matches(Token::Semicolon) {
                    Ok(Stmt::Return(None))
                } else {
                    let value = self.parse_expression()?;
                    self.expect(Token::Semicolon)?;
                    Ok(Stmt::Return(Some(value)))
                }
            }
            Token::Print => {
                let value = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Print(value))
            }
            Token::Input => {
                let value = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Input(value))
            }
            Token::FileOpen => {
                let filename = self.parse_expression()?;
                let mode = if self.matches(Token::Comma) {
                    let mode = self.parse_expression()?;
                    Self::expr_to_string(mode).to_lowercase()
                } else {
                    "read".to_string()
                };
                self.expect(Token::Semicolon)?;
                Ok(Stmt::FileOpen { filename, mode })
            }
            Token::FileClose => {
                let filename = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::FileClose { filename })
            }
            Token::FileWrite => {
                let filename = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let data = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::FileWrite { filename, data })
            }
            Token::FileRead => {
                let filename = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let variable = self.take_name("a variable to read into")?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::FileRead { filename, variable })
            }
            Token::ReadCSV => {
                let filename = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let variable = self.take_name("a variable to read into")?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::ReadCSV { filename, variable })
            }
            Token::WriteCSV => {
                let filename = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let data = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::WriteCSV { filename, data })
            }
            Token::DBConnect => {
                // The database type is the host's name for a backend, not a
                // name the author invented, so it keeps its canonical spelling.
                let db_type = Self::token_name(&self.take("a database type")?.token);
                self.expect(Token::Comma)?;
                let connection_string = self.parse_expression()?;
                // An optional name for this connection, so a second one can be
                // opened and told apart. Trailing, because the grammar is
                // fixed-arity and a trailing name therefore cannot be mistaken
                // for anything else.
                let handle = self.optional_handle("a name for this connection")?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::DBConnect {
                    db_type,
                    connection_string,
                    handle,
                })
            }
            Token::DBDisconnect => {
                let db_type = Self::token_name(&self.take("a database type")?.token);
                self.expect(Token::Semicolon)?;
                Ok(Stmt::DBDisconnect { db_type })
            }
            Token::DBQuery => {
                let query = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let params = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let result_var = self.take_name("a variable to hold the rows")?;
                let handle = self.optional_handle("the connection to query")?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::DBQuery {
                    query,
                    params,
                    result_var,
                    handle,
                })
            }
            Token::DBExecute => {
                let command = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let params = self.parse_expression()?;
                let handle = self.optional_handle("the connection to run this on")?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::DBExecute {
                    command,
                    params,
                    handle,
                })
            }
            Token::DBInsert => {
                let table = self.take_name("a table name")?;
                self.expect(Token::Comma)?;
                let data = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::DBInsert { table, data })
            }
            Token::DBUpdate => {
                let table = self.take_name("a table name")?;
                self.expect(Token::Comma)?;
                let data = self.parse_expression()?;
                let condition = if self.matches(Token::Comma) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect(Token::Semicolon)?;
                Ok(Stmt::DBUpdate {
                    table,
                    data,
                    condition,
                })
            }
            Token::DBDelete => {
                let table = self.take_name("a table name")?;
                self.expect(Token::Comma)?;
                let condition = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::DBDelete { table, condition })
            }
            Token::CreateTable => {
                let table = self.take_name("a table name")?;
                self.expect(Token::Comma)?;
                let schema = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::CreateTable { table, schema })
            }
            Token::Select => {
                let mut columns = Vec::new();
                loop {
                    columns.push(self.take_name("a column name")?);
                    if !self.matches(Token::Comma) {
                        break;
                    }
                }
                self.expect(Token::From)?;
                let from_table = self.take_name("a table name")?;
                let where_clause = if self.matches(Token::Where) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Select {
                    columns,
                    from_table,
                    where_clause,
                })
            }
            Token::Route => {
                // The HTTP method is matched by the router, so like a database
                // type it keeps its canonical spelling rather than the
                // author's.
                let method = Self::token_name(&self.take("an HTTP method")?.token);
                self.expect(Token::Comma)?;
                let path = self.parse_head()?;
                self.expect(Token::LBrace)?;
                let handler = self.parse_block()?;
                Ok(Stmt::DefineRoute {
                    method,
                    path,
                    handler,
                })
            }
            Token::Every => {
                let seconds = self.parse_head()?;
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                Ok(Stmt::Schedule { seconds, body })
            }
            Token::StartServer => {
                let host = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let port = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::StartServer { host, port })
            }
            Token::StopServer => {
                self.expect(Token::Semicolon)?;
                Ok(Stmt::StopServer)
            }
            Token::Response => {
                let status_code = self.parse_expression()?;
                self.expect(Token::Comma)?;
                let body = self.parse_expression()?;
                let headers = if self.matches(Token::Comma) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect(Token::Semicolon)?;
                Ok(Stmt::SendResponse {
                    status_code,
                    body,
                    headers,
                })
            }
            Token::JSONBody => {
                let data = self.parse_expression()?;
                let status_code = if self.matches(Token::Comma) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect(Token::Semicolon)?;
                Ok(Stmt::SendJSON { data, status_code })
            }
            Token::LParen => {
                let condition = self.parse_expression()?;
                self.expect(Token::RParen)?;

                let keyword = self.take("எனில் (eZil) or சுற்று (cuRRu) after a condition")?;
                match keyword.token {
                    Token::If => self.parse_if_remainder(condition),
                    Token::Loop => self.parse_loop_remainder(condition),
                    _ => {
                        Err(self
                            .mismatch(keyword, "எனில் (eZil) or சுற்று (cuRRu) after a condition"))
                    }
                }
            }
            // `வரிசை = 5;` is someone naming a variable, not writing an
            // unrecognisable statement. Reported as "a statement was expected"
            // it names neither the problem nor the remedy, and this is the
            // shape the mistake actually takes.
            _ if !Self::is_identifier_like(&current.token)
                && matches!(self.tokens.peek(), Some(next) if next.token == Token::Assign) =>
            {
                Err(self.mismatch(current, "a variable name"))
            }
            _ => Err(self.mismatch(current, "a statement")),
        }
    }

    /// Does the token after a `நிலை` make it the start of an immutable binding?
    ///
    /// A name or a type keyword does. Anything else — `=`, `.`, `[`, `(`, `,`
    /// — means நிலை is being used as the ordinary name it has always been.
    fn starts_fixed_binding(&mut self) -> bool {
        match self.peek_token() {
            Some(Token::Function) => true,
            Some(token) => Self::is_type_token(token) || Self::is_identifier_like(token),
            None => false,
        }
    }

    /// The rest of `நிலை [வகை] பெயர் = மதிப்பு;`, the `நிலை` already consumed.
    ///
    /// Unlike a plain declaration, the value is not optional. Rust would let a
    /// `let` be assigned later, once; here a name with no value would be a
    /// zero that can never become anything else, which is not a binding
    /// anyone means to write.
    fn parse_fixed_binding(&mut self) -> Result<Stmt, ParseError> {
        let declared = match self.peek_token() {
            Some(token) if Self::is_type_token(token) => {
                let token = self.take("a type")?;
                Some(Self::type_of(&token.token))
            }
            Some(Token::Function) => {
                self.advance();
                Some(DeclaredType::Function)
            }
            Some(token)
                if Self::is_plain_name(token)
                    && self.peek_after().is_some_and(Self::is_plain_name) =>
            {
                let spanned = self.advance().expect("peeked");
                Some(DeclaredType::Shape(self.name_of(spanned)))
            }
            _ => None,
        };

        let spanned = self.take("a name after நிலை (nilY)")?;
        if !Self::is_identifier_like(&spanned.token) || Self::is_type_token(&spanned.token) {
            return Err(self.mismatch(spanned, "a name after நிலை (nilY)"));
        }
        let name = self.name_of(spanned);
        let at = Position {
            line: spanned.line,
            column: spanned.column,
        };

        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect(Token::Semicolon)?;
        Ok(Stmt::Assign {
            name,
            value,
            declared,
            at,
            immutable: true,
        })
    }

    /// `(params) [return type]`, up to the `{` that opens a body.
    ///
    /// Shared by a named `செயல்` and one written as a value, which differ only
    /// in whether a name comes first.
    fn parse_signature(&mut self) -> Result<(Vec<Param>, Option<DeclaredType>), ParseError> {
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if !self.matches(Token::RParen) {
            loop {
                params.push(self.parse_param()?);
                if !self.matches(Token::Comma) {
                    break;
                }
            }
            self.expect(Token::RParen)?;
        }

        // An optional return type, between the parameter list and the body.
        // Nothing else can appear there, so it needs no marker. `செயல்` there
        // is a function that returns a function, and a name is a shape.
        let returns = match self.peek_token() {
            Some(token) if Self::is_type_token(token) => {
                let token = self.take("a return type")?;
                Some(Self::type_of(&token.token))
            }
            Some(Token::Function) if self.peek_after() == Some(&Token::LBrace) => {
                self.advance();
                Some(DeclaredType::Function)
            }
            Some(token)
                if Self::is_plain_name(token) && self.peek_after() == Some(&Token::LBrace) =>
            {
                let spanned = self.advance().expect("peeked");
                Some(DeclaredType::Shape(self.name_of(spanned)))
            }
            _ => None,
        };
        Ok((params, returns))
    }

    /// Statements up to a closing brace, which is consumed.
    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut body = Vec::new();
        loop {
            if self.matches(Token::RBrace) {
                return Ok(body);
            }
            if self.tokens.peek().is_none() {
                return Err(self.at_end("'}'"));
            }
            body.push(self.parse_statement()?);
        }
    }

    fn parse_if_remainder(&mut self, condition: Expr) -> Result<Stmt, ParseError> {
        self.expect(Token::LBrace)?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.matches(Token::Else) {
            self.expect(Token::LBrace)?;
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_loop_remainder(&mut self, condition: Expr) -> Result<Stmt, ParseError> {
        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        Ok(Stmt::Loop { condition, body })
    }

    // --- Expressions -------------------------------------------------------
    //
    // Precedence, loosest first:
    //   or < and < not < comparison < additive < term < factor

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.peek_token() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Logical {
                op: "||".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_not()?;
        while self.peek_token() == Some(&Token::And) {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::Logical {
                op: "&&".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, ParseError> {
        if self.peek_token() == Some(&Token::Not) {
            self.advance();
            return Ok(Expr::Not(Box::new(self.parse_not()?)));
        }
        self.parse_comparison()
    }

    /// One comparison, not a chain.
    ///
    /// This used to loop, which made comparison left-associative: `அ > ஆ > இ`
    /// parsed as `(அ > ஆ) > இ`, a Boolean compared against a number. `3 > 2 > 1`
    /// was **false**, and nothing said so.
    ///
    /// A second comparison operator is now an error rather than that. Chaining
    /// it the way Python does — `அ > ஆ மற்றும் ஆ > இ` — is what a language full
    /// of tax slabs actually wants, and it is the obvious next step. It needs
    /// the middle operand evaluated exactly once, and there is no way to say
    /// "once" in this AST, so `f() > g() > h()` would call `g` twice. Trading a
    /// wrong answer for a subtler wrong answer is not a fix.
    /// A comparison, or a chain of them.
    ///
    /// `300000 < வருமானம் <= 700000` is how a tax slab reads, and this language
    /// is full of tax slabs. It used to parse left-associatively as
    /// `(300000 < வருமானம்) <= 700000` — a boolean compared against a number —
    /// so `3 > 2 > 1` was **false** and nothing said so.
    ///
    /// A chain becomes `மற்றும்` over neighbouring pairs, which short-circuits
    /// and needs no new AST node. The one cost is that each middle operand is
    /// written twice, so it is only accepted where reading it twice does
    /// nothing twice — see `is_repeatable`. A middle operand that is a call is
    /// refused rather than quietly called twice.
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut operands = vec![self.parse_additive()?];
        let mut ops: Vec<&'static str> = Vec::new();

        loop {
            let op = match self.peek_token() {
                Some(Token::GreaterThan) => ">",
                Some(Token::LessThan) => "<",
                Some(Token::Equals) => "==",
                Some(Token::NotEquals) => "!=",
                Some(Token::GreaterThanOrEqual) => ">=",
                Some(Token::LessThanOrEqual) => "<=",
                _ => break,
            };
            let spanned = self.peek_spanned().cloned();
            self.advance();
            operands.push(self.parse_additive()?);
            ops.push(op);

            // Every operand between two operators is read twice. A name or a
            // literal does not mind; a call would happen twice, and a chain
            // that calls something twice is worse than one that will not
            // compile.
            if ops.len() > 1 {
                let middle = &operands[operands.len() - 2];
                if !Self::is_repeatable(middle) {
                    let expected = "தொடரின் நடுவில் ஒரு பெயர் அல்லது மாறிலி  \
                                    (the middle of a chain must be a name or a literal, \
                                    because it is compared twice: put the value in a \
                                    variable first)";
                    return Err(match spanned {
                        Some(spanned) => self.mismatch(&spanned, expected),
                        None => self.at_end(expected),
                    });
                }
            }
        }

        if ops.is_empty() {
            return Ok(operands.pop().expect("one operand"));
        }

        // One comparison keeps exactly the shape it always had, so nothing
        // downstream sees a difference for the ordinary case.
        let mut pairs = ops.iter().enumerate().map(|(index, op)| Expr::Comparison {
            left: Box::new(operands[index].clone()),
            op: op.to_string(),
            right: Box::new(operands[index + 1].clone()),
        });

        let first = pairs.next().expect("at least one pair");
        Ok(pairs.fold(first, |left, right| Expr::Logical {
            op: "&&".to_string(),
            left: Box::new(left),
            right: Box::new(right),
        }))
    }

    /// Can this expression be evaluated twice without doing anything twice?
    ///
    /// Reading a name, a literal, a field or an index is a read. A call is not,
    /// and neither is anything containing one, so `?` and arithmetic over a
    /// call are excluded along with the call itself.
    fn is_repeatable(expr: &Expr) -> bool {
        match expr {
            Expr::Variable(_)
            | Expr::Number(_)
            | Expr::String(_)
            | Expr::Boolean(_)
            | Expr::Null => true,
            Expr::Field { base, .. } => Self::is_repeatable(base),
            Expr::Index { base, index } => Self::is_repeatable(base) && Self::is_repeatable(index),
            _ => false,
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.peek_token() {
                Some(Token::Plus) => "+",
                Some(Token::Minus) => "-",
                Some(Token::Ampersand) => "&",
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            left = if op == "&" {
                Expr::Concat {
                    left: Box::new(left),
                    right: Box::new(right),
                }
            } else {
                Expr::BinaryOp {
                    op: op.to_string(),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match self.peek_token() {
                Some(Token::Multiply) => "*",
                Some(Token::Divide) => "/",
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::BinaryOp {
                op: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// A primary expression followed by any number of `[i]`, `.name`, `?`
    /// and `(args)`.
    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_primary()?;
        self.parse_postfix(expr)
    }

    /// Whatever follows a value: indexing, a field, `?`, or a call of it.
    fn parse_postfix(&mut self, mut expr: Expr) -> Result<Expr, ParseError> {
        loop {
            if self.matches(Token::LBracket) {
                let index = self.unrestricted(|parser| parser.parse_expression())?;
                self.expect(Token::RBracket)?;
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.matches(Token::Dot) {
                let at = match self.peek_spanned() {
                    Some(spanned) => Position {
                        line: spanned.line,
                        column: spanned.column,
                    },
                    None => Position {
                        line: self.last.0,
                        column: self.last.1,
                    },
                };
                let name = self.take_name("a field name")?;
                // `r.m(x)` is one thing, not a field read and then a call:
                // the method of r's shape comes first, and a function held in
                // the field only if there is none.
                expr = if self.matches(Token::LParen) {
                    let args = self.parse_arguments()?;
                    Expr::MethodCall {
                        receiver: Box::new(expr),
                        name,
                        args,
                        at,
                    }
                } else {
                    Expr::Field {
                        base: Box::new(expr),
                        name,
                        at,
                    }
                };
            } else if self.matches(Token::Question) {
                expr = Expr::Try(Box::new(expr));
            } else if self.matches(Token::LParen) {
                // `f(1)(2)`, `விதிகள்[0](தொகை)` — whatever came before is the
                // function. A bare name followed by `(` never reaches here:
                // that is an ordinary call, parsed with the name.
                let args = self.parse_arguments()?;
                expr = Expr::CallValue {
                    callee: Box::new(expr),
                    args,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Arguments after a `(` that has already been consumed, up to and
    /// including the `)`.
    fn parse_arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.unrestricted(|parser| {
            let mut args = Vec::new();
            if !parser.matches(Token::RParen) {
                loop {
                    args.push(parser.parse_expression()?);
                    if !parser.matches(Token::Comma) {
                        break;
                    }
                }
                parser.expect(Token::RParen)?;
            }
            Ok(args)
        })
    }

    /// The rest of `பெயர் { புலம்: மதிப்பு, …, ..அடிப்படை }`, the name and the
    /// `{` already consumed.
    fn parse_shape_literal(&mut self, shape: String, at: Position) -> Result<Expr, ParseError> {
        self.unrestricted(|parser| {
            let mut fields = Vec::new();
            let mut base = None;
            if !parser.matches(Token::RBrace) {
                loop {
                    // `..பழையது` — the rest of the fields from another record
                    // of this shape. Last, as in Rust, because it fills in
                    // what the fields before it did not say.
                    if parser.peek_token() == Some(&Token::Dot)
                        && parser.peek_after() == Some(&Token::Dot)
                    {
                        parser.advance();
                        parser.advance();
                        base = Some(Box::new(parser.parse_expression()?));
                        break;
                    }
                    let key = parser.take_name("a field name")?;
                    parser.expect(Token::Colon)?;
                    let value = parser.parse_expression()?;
                    fields.push((key, value));
                    if !parser.matches(Token::Comma) {
                        break;
                    }
                    if parser.peek_token() == Some(&Token::RBrace) {
                        break;
                    }
                }
                parser.expect(Token::RBrace)?;
            }
            Ok(Expr::ShapeLiteral {
                shape,
                fields,
                base,
                at,
            })
        })
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        // `இல்லை` as an operand, so that `"விடை: " & இல்லை காலியா(அ)` parses.
        // It was reachable only from `parse_not`, which sits above comparison
        // and therefore above `&`, so this was a parse error rather than an
        // expression. Binding it tightly here changes no existing meaning:
        // `இல்லை அ > ஆ` still goes through `parse_not` first, and still means
        // `இல்லை (அ > ஆ)` rather than `(இல்லை அ) > ஆ`.
        if self.peek_token() == Some(&Token::Not) {
            self.advance();
            return Ok(Expr::Not(Box::new(self.parse_factor()?)));
        }

        let spanned = self.take("a value")?;

        match &spanned.token {
            // அணி — an array literal: [a, b, c]
            Token::LBracket => self.unrestricted(|parser| {
                let mut items = Vec::new();
                if !parser.matches(Token::RBracket) {
                    loop {
                        items.push(parser.parse_expression()?);
                        if !parser.matches(Token::Comma) {
                            break;
                        }
                    }
                    parser.expect(Token::RBracket)?;
                }
                Ok(Expr::ArrayLiteral(items))
            }),
            // பொருள் — a record literal: {peyar: "ravi", vayaqu: 20}
            Token::LBrace => self.unrestricted(|parser| {
                let mut fields = Vec::new();
                if !parser.matches(Token::RBrace) {
                    loop {
                        let key = parser.take_name("a field name")?;
                        parser.expect(Token::Colon)?;
                        let value = parser.parse_expression()?;
                        fields.push((key, value));
                        if !parser.matches(Token::Comma) {
                            break;
                        }
                    }
                    parser.expect(Token::RBrace)?;
                }
                Ok(Expr::RecordLiteral(fields))
            }),
            // Unary minus, compiled as 0 - x.
            Token::Minus => {
                let operand = self.parse_factor()?;
                Ok(Expr::BinaryOp {
                    op: "-".to_string(),
                    left: Box::new(Expr::Number(Decimal::ZERO)),
                    right: Box::new(operand),
                })
            }
            // `செயல்(x) { … }` — a function written where a value goes. The
            // same signature as a named one, and no name.
            Token::Function => {
                let at = Position {
                    line: spanned.line,
                    column: spanned.column,
                };
                let (params, returns) = self.parse_signature()?;
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                Ok(Expr::Lambda {
                    params,
                    returns,
                    body,
                    at,
                })
            }
            Token::Number(n) => Ok(Expr::Number(*n)),
            Token::Percentage(n) => Ok(Expr::Number(*n)),
            Token::String(s) => Ok(Expr::String(s.clone())),
            Token::True => Ok(Expr::Boolean(true)),
            Token::False => Ok(Expr::Boolean(false)),
            Token::Null => Ok(Expr::Null),
            Token::LParen => {
                let expr = self.unrestricted(|parser| parser.parse_expression())?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            // An identifier, or a financial keyword used as a name.
            token if Self::is_identifier_like(token) && !Self::is_type_token(token) => {
                let name = self.name_of(spanned);
                // `கடன்{…}` — a shaped record, unless a block follows here.
                if !self.restrict && self.peek_token() == Some(&Token::LBrace) {
                    self.advance();
                    let at = Position {
                        line: spanned.line,
                        column: spanned.column,
                    };
                    return self.parse_shape_literal(name, at);
                }
                self.finish_name_or_call(name)
            }
            _ => Err(self.mismatch(spanned, "a value")),
        }
    }

    /// A name already consumed: a call if `(` follows, otherwise a variable.
    fn finish_name_or_call(&mut self, name: String) -> Result<Expr, ParseError> {
        if !self.matches(Token::LParen) {
            return Ok(Expr::Variable(name));
        }
        let args = self.parse_arguments()?;
        Ok(Expr::Call { name, args })
    }

    // --- Token classification ---------------------------------------------

    // rustfmt is off here on purpose: scripts/generate_editor_support.py reads
    // these arms a line at a time, and wrapping one across lines makes it
    // invisible to the generator — which then writes editor data missing
    // whatever it could not see, and CI gates on that file.
    #[rustfmt::skip]
    fn is_identifier_like(token: &Token) -> bool {
        match token {
            Token::Number(_) | Token::Percentage(_) | Token::String(_) => false,
            Token::If | Token::Else | Token::Loop | Token::Print | Token::Input => false,
            Token::And | Token::Or | Token::Not => false,
            Token::True | Token::False | Token::Null => false,
            Token::Function | Token::Return => false,
            Token::ForEach | Token::In | Token::Import => false,
            Token::Assign | Token::Plus | Token::Minus | Token::Multiply | Token::Divide | Token::Ampersand => false,
            Token::Question | Token::Dot | Token::Colon => false,
            Token::LBracket | Token::RBracket => false,
            Token::LParen | Token::RParen | Token::LBrace | Token::RBrace | Token::Comma | Token::Semicolon => false,
            Token::GreaterThan | Token::LessThan | Token::Equals | Token::NotEquals | Token::GreaterThanOrEqual | Token::LessThanOrEqual => false,
            Token::File | Token::CSV | Token::Read | Token::Write | Token::Open | Token::Close => false,
            Token::FileOpen | Token::FileClose | Token::FileRead | Token::FileWrite | Token::ReadCSV | Token::WriteCSV => false,
            // Database operations
            Token::Database | Token::DBConnect | Token::DBDisconnect | Token::DBQuery | Token::DBExecute | Token::DBSearch | Token::DBInsert | Token::DBUpdate | Token::DBDelete => false,
            Token::Table | Token::Collection | Token::Row | Token::Column | Token::Key | Token::PrimaryKey | Token::ForeignKey | Token::Index => false,
            Token::CreateTable | Token::AlterTable | Token::DropTable => false,
            Token::Select | Token::From | Token::Where | Token::OrderBy | Token::GroupBy | Token::Join | Token::Left | Token::Right | Token::Inner | Token::Outer | Token::Distinct | Token::Limit | Token::Offset => false,
            // Database types
            Token::SQL | Token::NoSQL | Token::SQLite | Token::MySQL | Token::PostgreSQL | Token::MongoDB | Token::Redis | Token::JSONdb => false,
            // REST API operations
            Token::Route | Token::Every | Token::StartServer | Token::StopServer | Token::Response | Token::JSONBody => false,
            Token::Request | Token::Endpoint | Token::API | Token::Header | Token::Body | Token::Param | Token::QueryParam | Token::PathParam => false,
            Token::URL | Token::Host | Token::Port | Token::Method | Token::StatusCode | Token::StatusMessage | Token::Auth | Token::BearerToken | Token::ContentType | Token::Serve => false,
            Token::HttpGet | Token::HttpPost | Token::HttpPut | Token::HttpDelete | Token::HttpPatch | Token::HttpOptions | Token::HttpHead => false,
            // Financial and accounting keywords ARE usable as names: வருவாய்,
            // வரி and the rest are the domain nouns programs are written
            // about. They have no statement syntax of their own, and listing
            // them here made `எண் வருவாய்;` — the language's own headline
            // example — a parse error.
            _ => true,
        }
    }

    // rustfmt is off here on purpose: scripts/generate_editor_support.py reads
    // these arms a line at a time, and wrapping one across lines makes it
    // invisible to the generator — which then writes editor data missing
    // whatever it could not see, and CI gates on that file.
    #[rustfmt::skip]
    fn is_type_token(token: &Token) -> bool {
        matches!(
            token,
            Token::IntegerType
                | Token::FloatType
                | Token::StringType
                | Token::BoolType
                | Token::TextType
                | Token::ArrayType
                | Token::DataType
                | Token::ObjectType
                | Token::DateType
        )
    }

    /// One parameter: an optional type keyword, then the name. The same order
    /// a variable declaration uses, so `எண் தொகை` reads the same in both places.
    fn parse_param(&mut self) -> Result<Param, ParseError> {
        // `நிலை எண் தொகை` — a parameter the body cannot assign to. A parameter
        // that is itself called நிலை is followed by `,` or `)` instead.
        let immutable = self.peek_token() == Some(&Token::Const)
            && matches!(
                self.peek_after(),
                Some(token) if Self::is_type_token(token) || Self::is_identifier_like(token)
            );
        if immutable {
            self.advance();
        }

        let declared = match self.peek_token() {
            Some(token) if Self::is_type_token(token) => {
                let token = self.take("a parameter type")?;
                Some(Self::type_of(&token.token))
            }
            // `செயல் மாற்று` — a parameter that must be a function.
            Some(Token::Function) => {
                self.advance();
                Some(DeclaredType::Function)
            }
            // `கடன் க` — a parameter that must be a record of that shape.
            Some(token)
                if Self::is_plain_name(token)
                    && self.peek_after().is_some_and(Self::is_plain_name) =>
            {
                let spanned = self.advance().expect("peeked");
                Some(DeclaredType::Shape(self.name_of(spanned)))
            }
            _ => None,
        };

        let spanned = self.take("a parameter name")?;
        if !Self::is_identifier_like(&spanned.token) || Self::is_type_token(&spanned.token) {
            return Err(self.mismatch(spanned, "a parameter name"));
        }
        let at = Position {
            line: spanned.line,
            column: spanned.column,
        };
        Ok(Param {
            name: self.name_of(spanned),
            declared,
            immutable,
            at,
        })
    }

    /// The declared type a type keyword names.
    fn type_of(token: &Token) -> DeclaredType {
        match token {
            Token::IntegerType | Token::FloatType => DeclaredType::Number,
            Token::StringType | Token::TextType => DeclaredType::Text,
            Token::BoolType => DeclaredType::Boolean,
            Token::ArrayType => DeclaredType::Array,
            Token::ObjectType | Token::DataType => DeclaredType::Record,
            Token::DateType => DeclaredType::Date,
            // is_type_token gates every caller, so this is unreachable in
            // practice; treated as unconstrained rather than panicking.
            _ => DeclaredType::Any,
        }
    }

    fn expr_to_string(expr: Expr) -> String {
        match expr {
            Expr::String(s) => s,
            Expr::Variable(name) => name,
            Expr::Number(n) => n.to_string(),
            Expr::Boolean(b) => b.to_string(),
            Expr::Null => "nil".to_string(),
            Expr::BinaryOp { op, .. } => op,
            Expr::Comparison { op, .. } => op,
            Expr::Logical { op, .. } => op,
            Expr::Not(_) => "not".to_string(),
            Expr::Call { name, .. } => name,
            Expr::ArrayLiteral(_) => "array".to_string(),
            Expr::RecordLiteral(_) => "record".to_string(),
            Expr::Index { .. } => "index".to_string(),
            Expr::Field { name, .. } => name,
            Expr::Try(_) => "try".to_string(),
            Expr::Lambda { .. } => "function".to_string(),
            Expr::CallValue { .. } => "call".to_string(),
            Expr::MethodCall { name, .. } => name,
            Expr::ShapeLiteral { shape, .. } => shape,
            Expr::Concat { .. } => "concat".to_string(),
        }
    }
}

// --- Names: what a statement binds, and what a body reads -------------------
//
// Both backends decide what an anonymous செயல் captures, and they must decide
// it the same way or a compiled program would see a different value from the
// interpreted one. So the decision is made here, from the AST, once.

/// The names a statement binds in the scope it runs in, in order.
///
/// Only the statement itself: a name bound inside an `எனில்` branch is bound by
/// that inner statement, which the caller reaches as it walks the branch. The
/// kinds are exactly those that give a name a value in both backends.
pub fn bound_names(statement: &Stmt) -> Vec<&str> {
    match statement {
        Stmt::Assign { name, .. } => vec![name],
        Stmt::ForEach { var, .. } => vec![var],
        Stmt::Input(Expr::Variable(name)) => vec![name],
        Stmt::DBQuery { result_var, .. } => vec![result_var],
        Stmt::FileRead { variable, .. }
        | Stmt::ReadCSV { variable, .. }
        | Stmt::GetRequestBody { variable }
        | Stmt::GetRequestParam { variable, .. }
        | Stmt::GetHeader { variable, .. } => vec![variable],
        _ => Vec::new(),
    }
}

/// What an anonymous செயல் carries away: the names its body reads that are
/// locals where it was written, sorted so both backends order them alike.
///
/// `local` says whether a name is a local at the point the செயல் appears. A
/// name read inside a செயல் nested in this one counts, because the inner one
/// can only capture what the outer one has.
pub fn captures(params: &[Param], body: &[Stmt], local: impl Fn(&str) -> bool) -> Vec<String> {
    let mut read = std::collections::BTreeSet::new();
    for statement in body {
        stmt_reads(statement, &mut read);
    }
    read.into_iter()
        .filter(|name| !params.iter().any(|param| &param.name == name))
        .filter(|name| local(name))
        .collect()
}

fn stmt_reads(statement: &Stmt, into: &mut std::collections::BTreeSet<String>) {
    let expr = expr_reads;
    match statement {
        Stmt::Assign { value, .. } => expr(value, into),
        // A named செயல் captures nothing, so what it reads is not this
        // function's business.
        Stmt::FunctionDef { .. } => {}
        Stmt::Return(Some(value)) | Stmt::Expression(value) | Stmt::Print(value) => {
            expr(value, into)
        }
        Stmt::SetIndex {
            name, index, value, ..
        } => {
            into.insert(name.clone());
            expr(index, into);
            expr(value, into);
        }
        Stmt::SetField { name, value, .. } => {
            into.insert(name.clone());
            expr(value, into);
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr(condition, into);
            for inner in then_branch.iter().chain(else_branch.iter().flatten()) {
                stmt_reads(inner, into);
            }
        }
        Stmt::Loop { condition, body } => {
            expr(condition, into);
            for inner in body {
                stmt_reads(inner, into);
            }
        }
        Stmt::ForEach {
            collection, body, ..
        } => {
            expr(collection, into);
            for inner in body {
                stmt_reads(inner, into);
            }
        }
        Stmt::FileOpen { filename, .. }
        | Stmt::FileClose { filename }
        | Stmt::FileRead { filename, .. }
        | Stmt::ReadCSV { filename, .. } => expr(filename, into),
        Stmt::FileWrite { filename, data } | Stmt::WriteCSV { filename, data } => {
            expr(filename, into);
            expr(data, into);
        }
        Stmt::DBConnect {
            connection_string, ..
        } => expr(connection_string, into),
        Stmt::DBQuery { query, params, .. } => {
            expr(query, into);
            expr(params, into);
        }
        Stmt::DBExecute {
            command, params, ..
        } => {
            expr(command, into);
            expr(params, into);
        }
        Stmt::SendResponse {
            status_code,
            body,
            headers,
        } => {
            expr(status_code, into);
            expr(body, into);
            if let Some(headers) = headers {
                expr(headers, into);
            }
        }
        Stmt::SendJSON { data, status_code } => {
            expr(data, into);
            if let Some(status) = status_code {
                expr(status, into);
            }
        }
        _ => {}
    }
}

fn expr_reads(expr: &Expr, into: &mut std::collections::BTreeSet<String>) {
    match expr {
        Expr::Variable(name) => {
            into.insert(name.clone());
        }
        // A call by name may be a call of a local holding a function.
        Expr::Call { name, args } => {
            into.insert(name.clone());
            for arg in args {
                expr_reads(arg, into);
            }
        }
        Expr::CallValue { callee, args } => {
            expr_reads(callee, into);
            for arg in args {
                expr_reads(arg, into);
            }
        }
        Expr::BinaryOp { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Concat { left, right }
        | Expr::Logical { left, right, .. } => {
            expr_reads(left, into);
            expr_reads(right, into);
        }
        Expr::Not(inner) | Expr::Try(inner) => expr_reads(inner, into),
        Expr::ArrayLiteral(items) => {
            for item in items {
                expr_reads(item, into);
            }
        }
        Expr::RecordLiteral(fields) => {
            for (_, value) in fields {
                expr_reads(value, into);
            }
        }
        Expr::Index { base, index } => {
            expr_reads(base, into);
            expr_reads(index, into);
        }
        Expr::Field { base, .. } => expr_reads(base, into),
        Expr::MethodCall { receiver, args, .. } => {
            expr_reads(receiver, into);
            for arg in args {
                expr_reads(arg, into);
            }
        }
        Expr::ShapeLiteral { fields, base, .. } => {
            for (_, value) in fields {
                expr_reads(value, into);
            }
            if let Some(base) = base {
                expr_reads(base, into);
            }
        }
        // What a nested செயல் reads, less its own parameters, has to be
        // available to it — so this one must carry it too.
        Expr::Lambda { params, body, .. } => {
            let mut inner = std::collections::BTreeSet::new();
            for statement in body {
                stmt_reads(statement, &mut inner);
            }
            for name in inner {
                if !params.iter().any(|param| param.name == name) {
                    into.insert(name);
                }
            }
        }
        Expr::Number(_) | Expr::String(_) | Expr::Boolean(_) | Expr::Null => {}
    }
}
