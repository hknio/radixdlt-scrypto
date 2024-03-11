use std::fmt::Display;

use annotate_snippets::{Annotation, AnnotationType, Renderer, Slice, Snippet, SourceAnnotation};
use sbor::rust::cmp::min;
use sbor::rust::fmt;
use sbor::rust::fmt::Debug;
use sbor::rust::str::FromStr;

/// The span of tokens. The `start` and `end` are Unicode code points / UTF-32 - as opposed to a
/// byte-based / UTF-8 index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The start of the span, exclusive
    pub start: Position,
    /// The end of the span, inclusive
    pub end: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// A 0-indexed cursor indicating the next unicode char from the start
    pub full_index: usize,
    /// A 1-indexed cursor indicating the line number (assuming \n is a line break)
    pub line_number: usize,
    /// A 0-indexed cursor indicating the character offset in the line
    pub line_char_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // ==============
    // Literals
    // ==============
    BoolLiteral(bool),
    I8Literal(i8),
    I16Literal(i16),
    I32Literal(i32),
    I64Literal(i64),
    I128Literal(i128),
    U8Literal(u8),
    U16Literal(u16),
    U32Literal(u32),
    U64Literal(u64),
    U128Literal(u128),
    StringLiteral(String),

    Ident(String),

    /* Punctuations */
    OpenParenthesis,
    CloseParenthesis,
    LessThan,
    GreaterThan,
    Comma,
    Semicolon,
    FatArrow,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenKind::BoolLiteral(bool) => write!(f, "`{:?}`", bool),
            TokenKind::I8Literal(i8) => write!(f, "`i8: {:?}`", i8),
            TokenKind::I16Literal(i16) => write!(f, "i16: `{:?}`", i16),
            TokenKind::I32Literal(i32) => write!(f, "i32: `{:?}`", i32),
            TokenKind::I64Literal(i64) => write!(f, "i64: `{:?}`", i64),
            TokenKind::I128Literal(i128) => write!(f, "i128: `{:?}`", i128),
            TokenKind::U8Literal(u8) => write!(f, "u8: `{:?}`", u8),
            TokenKind::U16Literal(u16) => write!(f, "u16: `{:?}`", u16),
            TokenKind::U32Literal(u32) => write!(f, "u32: `{:?}`", u32),
            TokenKind::U64Literal(u64) => write!(f, "u64: `{:?}`", u64),
            TokenKind::U128Literal(u128) => write!(f, "u128: `{:?}`", u128),
            TokenKind::StringLiteral(string) => write!(f, "String: `{}`", string),
            TokenKind::Ident(string) => write!(f, "Ident: `{}`", string),
            TokenKind::OpenParenthesis => write!(f, "token `(`"),
            TokenKind::CloseParenthesis => write!(f, "token `)`",),
            TokenKind::LessThan => write!(f, "token `<`"),
            TokenKind::GreaterThan => write!(f, "token `>`",),
            TokenKind::Comma => write!(f, "token `,`"),
            TokenKind::Semicolon => write!(f, "token `;`",),
            TokenKind::FatArrow => write!(f, "token `=>`"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexerError {
    UnexpectedEof,
    UnexpectedChar(char, Position),
    InvalidInteger(String, Span),
    InvalidUnicode(u32, Position),
    UnknownIdentifier(String, Position),
}

#[derive(Debug, Clone)]
pub struct Lexer {
    /// The input text chars
    text: Vec<char>,
    /// The current position in the text
    current: Position,
    /// The previous position in the text
    previous: Position,
    /// The start position of token being lexed
    start: Position,
}

pub fn tokenize(s: &str) -> Result<Vec<Token>, LexerError> {
    let mut lexer = Lexer::new(s);
    let mut tokens = Vec::new();
    loop {
        if let Some(token) = lexer.next_token()? {
            tokens.push(token);
        } else {
            break;
        }
    }
    Ok(tokens)
}

impl Lexer {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.chars().collect(),
            current: Position {
                full_index: 0,
                line_number: 1,
                line_char_index: 0,
            },
            previous: Position {
                full_index: 0,
                line_number: 1,
                line_char_index: 0,
            },
            start: Position {
                full_index: 0,
                line_number: 1,
                line_char_index: 0,
            },
        }
    }

    pub fn is_eof(&self) -> bool {
        self.current.full_index == self.text.len()
    }

    fn peek(&self) -> Result<char, LexerError> {
        self.text
            .get(self.current.full_index)
            .cloned()
            .ok_or(LexerError::UnexpectedEof)
    }

    fn advance(&mut self) -> Result<char, LexerError> {
        let c = self.peek()?;
        self.previous = self.current;
        self.current.full_index += 1;
        if c == '\n' {
            self.current.line_number += 1;
            self.current.line_char_index = 0;
        } else {
            self.current.line_char_index += 1;
        }
        Ok(c)
    }

    fn is_whitespace(c: char) -> bool {
        // slightly different from the original specs, we skip `\n`
        // rather than consider it as a terminal
        c == ' ' || c == '\t' || c == '\r' || c == '\n'
    }

    pub fn next_token(&mut self) -> Result<Option<Token>, LexerError> {
        // skip comment and whitespace
        let mut in_comment = false;
        while !self.is_eof() {
            if in_comment {
                if self.advance()? == '\n' {
                    in_comment = false;
                }
            } else if self.peek()? == '#' {
                in_comment = true;
            } else if Self::is_whitespace(self.peek()?) {
                self.advance()?;
            } else {
                break;
            }
        }

        // check if it's the end of file
        if self.is_eof() {
            return Ok(None);
        }

        // match next token
        match self.peek()? {
            '-' | '0'..='9' => self.tokenize_number(),
            '"' => self.tokenize_string(),
            'a'..='z' | 'A'..='Z' => self.tokenize_identifier(),
            '{' | '}' | '(' | ')' | '<' | '>' | ',' | ';' | '&' | '=' => {
                self.tokenize_punctuation()
            }
            _ => Err(LexerError::UnexpectedChar(
                self.text[self.current.full_index],
                self.current,
            )),
        }
        .map(Option::from)
    }

    // TODO: consider using DFA
    fn tokenize_number(&mut self) -> Result<Token, LexerError> {
        self.start = self.current;
        let mut s = String::new();

        // negative sign
        if self.peek()? == '-' {
            s.push(self.advance()?);
        }

        // integer
        match self.advance()? {
            c @ '0' => {
                s.push(c);
            }
            c @ '1'..='9' => {
                s.push(c);
                while self.peek()?.is_ascii_digit() {
                    s.push(self.advance()?);
                }
            }
            _ => {
                return Err(self.unexpected_char());
            }
        }

        // type
        match self.advance()? {
            'i' => match self.advance()? {
                '1' => match self.advance()? {
                    '2' => match self.advance()? {
                        '8' => self.parse_int(&s, "i128", TokenKind::I128Literal),
                        _ => Err(self.unexpected_char_previous()),
                    },
                    '6' => self.parse_int(&s, "i16", TokenKind::I16Literal),
                    _ => Err(self.unexpected_char_previous()),
                },
                '3' => match self.advance()? {
                    '2' => self.parse_int(&s, "i32", TokenKind::I32Literal),
                    _ => Err(self.unexpected_char_previous()),
                },
                '6' => match self.advance()? {
                    '4' => self.parse_int(&s, "i64", TokenKind::I64Literal),
                    _ => Err(self.unexpected_char_previous()),
                },
                '8' => self.parse_int(&s, "i8", TokenKind::I8Literal),
                _ => Err(self.unexpected_char_previous()),
            },
            'u' => match self.advance()? {
                '1' => match self.advance()? {
                    '2' => match self.advance()? {
                        '8' => self.parse_int(&s, "u128", TokenKind::U128Literal),
                        _ => Err(self.unexpected_char_previous()),
                    },
                    '6' => self.parse_int(&s, "u16", TokenKind::U16Literal),
                    _ => Err(self.unexpected_char_previous()),
                },
                '3' => match self.advance()? {
                    '2' => self.parse_int(&s, "u32", TokenKind::U32Literal),
                    _ => Err(self.unexpected_char_previous()),
                },
                '6' => match self.advance()? {
                    '4' => self.parse_int(&s, "u64", TokenKind::U64Literal),
                    _ => Err(self.unexpected_char_previous()),
                },
                '8' => self.parse_int(&s, "u8", TokenKind::U8Literal),
                _ => Err(self.unexpected_char_previous()),
            },
            _ => Err(self.unexpected_char_previous()),
        }
        .map(|kind| self.new_token(kind, self.start, self.current))
    }

    fn parse_int<T>(
        &self,
        int: &str,
        ty: &str,
        map: fn(T) -> TokenKind,
    ) -> Result<TokenKind, LexerError>
    where
        T: FromStr,
        <T as FromStr>::Err: Display,
    {
        int.parse::<T>().map(map).map_err(|err| {
            LexerError::InvalidInteger(
                format!("{} - {} {}", int, err.to_string(), ty),
                Span {
                    start: self.start,
                    end: self.current,
                },
            )
        })
    }

    fn tokenize_string(&mut self) -> Result<Token, LexerError> {
        let start = self.current;
        assert_eq!(self.advance()?, '"');

        let mut s = String::new();
        while self.peek()? != '"' {
            let c = self.advance()?;
            if c == '\\' {
                // See the JSON string specifications
                match self.advance()? {
                    '"' => s.push('\"'),
                    '\\' => s.push('\\'),
                    '/' => s.push('/'),
                    'b' => s.push('\x08'),
                    'f' => s.push('\x0c'),
                    'n' => s.push('\n'),
                    'r' => s.push('\r'),
                    't' => s.push('\t'),
                    'u' => {
                        let mut unicode = self.read_utf16_unit()?;
                        // Check unicode surrogate pair
                        // (see https://unicodebook.readthedocs.io/unicode_encodings.html#surrogates)
                        if (0xD800..=0xDFFF).contains(&unicode) {
                            if self.advance()? == '\\' && self.advance()? == 'u' {
                                unicode = 0x10000
                                    + ((unicode - 0xD800) << 10)
                                    + self.read_utf16_unit()?
                                    - 0xDC00;
                            } else {
                                return Err(self.unexpected_char());
                            }
                        }
                        s.push(
                            char::from_u32(unicode)
                                .ok_or(LexerError::InvalidUnicode(unicode, self.current))?,
                        );
                    }
                    _ => {
                        return Err(self.unexpected_char());
                    }
                }
            } else {
                s.push(c);
            }
        }
        self.advance()?;

        Ok(self.new_token(TokenKind::StringLiteral(s), start, self.current))
    }

    fn read_utf16_unit(&mut self) -> Result<u32, LexerError> {
        let mut code: u32 = 0;

        for _ in 0..4 {
            let c = self.advance()?;
            if c.is_ascii_hexdigit() {
                code = code * 16 + c.to_digit(16).unwrap();
            } else {
                return Err(self.unexpected_char());
            }
        }

        Ok(code)
    }

    fn tokenize_identifier(&mut self) -> Result<Token, LexerError> {
        let start = self.current;

        let mut id = String::from(self.advance()?);
        while !self.is_eof() {
            let next_char = self.peek()?;
            let next_char_can_be_part_of_ident =
                next_char.is_ascii_alphanumeric() || next_char == '_' || next_char == ':';
            if !next_char_can_be_part_of_ident {
                break;
            }
            id.push(self.advance()?);
        }

        let kind = match id.as_str() {
            "true" => TokenKind::BoolLiteral(true),
            "false" => TokenKind::BoolLiteral(false),
            other => TokenKind::Ident(other.to_string()),
        };
        Ok(self.new_token(kind, start, self.current))
    }

    fn tokenize_punctuation(&mut self) -> Result<Token, LexerError> {
        let start = self.current;

        let token_kind = match self.advance()? {
            '(' => TokenKind::OpenParenthesis,
            ')' => TokenKind::CloseParenthesis,
            '<' => TokenKind::LessThan,
            '>' => TokenKind::GreaterThan,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '=' => match self.advance()? {
                '>' => TokenKind::FatArrow,
                _ => return Err(self.unexpected_char_previous()),
            },
            _ => {
                return Err(self.unexpected_char_previous());
            }
        };

        Ok(self.new_token(token_kind, start, self.current))
    }

    fn new_token(&self, kind: TokenKind, start: Position, end: Position) -> Token {
        Token {
            kind,
            span: Span { start, end },
        }
    }

    fn unexpected_char(&self) -> LexerError {
        LexerError::UnexpectedChar(self.text[self.current.full_index - 1], self.current)
    }

    fn unexpected_char_previous(&self) -> LexerError {
        LexerError::UnexpectedChar(self.text[self.previous.full_index], self.previous)
    }
}

pub fn lexer_error_diagnostics(s: &str, err: LexerError) -> String {
    let lines_cnt = s.lines().count();
    let (mut span, title, label) = match err {
        LexerError::UnexpectedEof => (
            Span {
                start: Position {
                    full_index: s.len() - 1,
                    line_number: lines_cnt,
                    line_char_index: 0,
                },
                end: Position {
                    full_index: s.len() - 1,
                    line_number: lines_cnt,
                    line_char_index: 0,
                },
            },
            "unexpected end of file".to_string(),
            "end of file".to_string(),
        ),
        LexerError::UnexpectedChar(c, position) => {
            let mut end = position;
            end.full_index += 1;
            (
                Span {
                    start: position,
                    end,
                },
                format!("unexpected character {:?}", c),
                "unexpected character".to_string(),
            )
        }
        LexerError::InvalidInteger(string, span) => (
            span,
            format!("invalid integer {}", string),
            "invalid integer".to_string(),
        ),
        LexerError::InvalidUnicode(value, position) => (
            Span {
                start: position,
                end: position,
            },
            format!("invalid unicode value {}", value),
            "invalid unicode".to_string(),
        ),
        LexerError::UnknownIdentifier(string, position) => (
            Span {
                start: position,
                end: position,
            },
            format!("unknown identifier {}", string),
            "unknown identifier".to_string(),
        ),
    };

    // Surround span with few lines for more context
    if span.start.line_number > 5 {
        span.start.line_number -= 5;
    } else {
        span.start.line_number = 0;
    }
    span.end.line_number = min(span.end.line_number + 5, lines_cnt);

    let mut source = String::new();
    let mut skipped_chars = 0;
    for (i, line) in s.lines().enumerate() {
        if (i + 1) < span.start.line_number {
            // Add 1 for '\n' character
            skipped_chars += line.chars().count() + 1;
        } else if (i + 1) <= span.end.line_number {
            source.push_str(line.into());
            source.push('\n');
        } else if (i + 1) > span.end.line_number {
            break;
        }
    }

    span.start.full_index -= skipped_chars;
    span.end.full_index -= skipped_chars;

    let snippet = Snippet {
        slices: vec![Slice {
            source: source.as_str(),
            line_start: span.start.line_number + 1,
            origin: None,
            fold: false,
            annotations: vec![SourceAnnotation {
                label: label.as_str(),
                annotation_type: AnnotationType::Info,
                range: (span.start.full_index, span.end.full_index),
            }],
        }],
        title: Some(Annotation {
            label: Some(title.as_str()),
            id: None,
            annotation_type: AnnotationType::Error,
        }),
        footer: vec![],
    };

    let renderer = Renderer::styled();
    let s = renderer.render(snippet).to_string();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[macro_export]
    macro_rules! lex_ok {
        ( $s:expr, $expected:expr ) => {{
            let mut lexer = Lexer::new($s);
            for i in 0..$expected.len() {
                assert_eq!(
                    lexer.next_token().map(|opt| opt.map(|t| t.kind)),
                    Ok(Some($expected[i].clone()))
                );
            }
            assert_eq!(lexer.next_token(), Ok(None));
        }};
    }

    #[macro_export]
    macro_rules! lex_error {
        ( $s:expr, $expected:expr ) => {{
            let mut lexer = Lexer::new($s);
            loop {
                match lexer.next_token() {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        panic!("Expected {:?} but no error is thrown", $expected);
                    }
                    Err(e) => {
                        assert_eq!(e, $expected);
                        break;
                    }
                }
            }
        }};
    }

    #[test]
    fn test_empty_strings() {
        lex_ok!("", Vec::<TokenKind>::new());
        lex_ok!("  ", Vec::<TokenKind>::new());
        lex_ok!("\r\n\t", Vec::<TokenKind>::new());
    }

    #[test]
    fn test_bool() {
        lex_ok!("true", vec![TokenKind::BoolLiteral(true)]);
        lex_ok!("false", vec![TokenKind::BoolLiteral(false)]);
        lex_ok!("false123u8", vec![TokenKind::Ident("false123u8".into())]);
    }

    #[test]
    fn test_int() {
        lex_ok!(
            "1u82u1283i84i128",
            vec![
                TokenKind::U8Literal(1),
                TokenKind::U128Literal(2),
                TokenKind::I8Literal(3),
                TokenKind::I128Literal(4),
            ]
        );
        lex_ok!(
            "1u8 2u32",
            vec![TokenKind::U8Literal(1), TokenKind::U32Literal(2)]
        );
        lex_error!("123", LexerError::UnexpectedEof);
    }

    #[test]
    fn test_comment() {
        lex_ok!("# 1u8", Vec::<TokenKind>::new());
        lex_ok!("1u8 # comment", vec![TokenKind::U8Literal(1),]);
        lex_ok!(
            "# multiple\n# line\nCALL_FUNCTION",
            vec![TokenKind::Ident("CALL_FUNCTION".to_string()),]
        );
    }

    #[test]
    fn test_string() {
        lex_ok!(
            r#"  "" "abc" "abc\r\n\"def\uD83C\uDF0D"  "#,
            vec![
                TokenKind::StringLiteral("".into()),
                TokenKind::StringLiteral("abc".into()),
                TokenKind::StringLiteral("abc\r\n\"def🌍".into()),
            ]
        );
        lex_error!("\"", LexerError::UnexpectedEof);
    }

    #[test]
    fn test_mixed() {
        lex_ok!(
            r#"CALL_FUNCTION Map<String, Array>("test", Array<String>("abc"));"#,
            vec![
                TokenKind::Ident("CALL_FUNCTION".to_string()),
                TokenKind::Ident("Map".to_string()),
                TokenKind::LessThan,
                TokenKind::Ident("String".to_string()),
                TokenKind::Comma,
                TokenKind::Ident("Array".to_string()),
                TokenKind::GreaterThan,
                TokenKind::OpenParenthesis,
                TokenKind::StringLiteral("test".into()),
                TokenKind::Comma,
                TokenKind::Ident("Array".to_string()),
                TokenKind::LessThan,
                TokenKind::Ident("String".to_string()),
                TokenKind::GreaterThan,
                TokenKind::OpenParenthesis,
                TokenKind::StringLiteral("abc".into()),
                TokenKind::CloseParenthesis,
                TokenKind::CloseParenthesis,
                TokenKind::Semicolon,
            ]
        );
    }

    #[test]
    fn test_precise_decimal() {
        lex_ok!(
            "PreciseDecimal(\"12\")",
            vec![
                TokenKind::Ident("PreciseDecimal".to_string()),
                TokenKind::OpenParenthesis,
                TokenKind::StringLiteral("12".into()),
                TokenKind::CloseParenthesis,
            ]
        );
    }

    #[test]
    fn test_precise_decimal_collection() {
        lex_ok!(
            "Array<PreciseDecimal>(PreciseDecimal(\"12\"), PreciseDecimal(\"212\"), PreciseDecimal(\"1984\"))",
            vec![
                TokenKind::Ident("Array".to_string()),
                TokenKind::LessThan,
                TokenKind::Ident("PreciseDecimal".to_string()),
                TokenKind::GreaterThan,
                TokenKind::OpenParenthesis,
                TokenKind::Ident("PreciseDecimal".to_string()),
                TokenKind::OpenParenthesis,
                TokenKind::StringLiteral("12".into()),
                TokenKind::CloseParenthesis,
                TokenKind::Comma,
                TokenKind::Ident("PreciseDecimal".to_string()),
                TokenKind::OpenParenthesis,
                TokenKind::StringLiteral("212".into()),
                TokenKind::CloseParenthesis,
                TokenKind::Comma,
                TokenKind::Ident("PreciseDecimal".to_string()),
                TokenKind::OpenParenthesis,
                TokenKind::StringLiteral("1984".into()),
                TokenKind::CloseParenthesis,
                TokenKind::CloseParenthesis,
            ]
        );
    }

    #[test]
    fn test_unexpected_char() {
        lex_error!(
            "1u8 +2u32",
            LexerError::UnexpectedChar(
                '+',
                Position {
                    full_index: 4,
                    line_number: 1,
                    line_char_index: 4
                }
            )
        );

        lex_error!(
            "x=7",
            LexerError::UnexpectedChar(
                '7',
                Position {
                    full_index: 2,
                    line_number: 1,
                    line_char_index: 2
                }
            )
        );
        lex_error!(
            "1i128\n 1u64 \n 1i37",
            LexerError::UnexpectedChar(
                '7',
                Position {
                    full_index: 17,
                    line_number: 3,
                    line_char_index: 4
                }
            )
        );
        lex_error!(
            "3_0i8",
            LexerError::UnexpectedChar(
                '_',
                Position {
                    full_index: 1,
                    line_number: 1,
                    line_char_index: 1
                }
            )
        );
    }

    #[test]
    fn test_unicode() {
        lex_ok!(
            r#""\u2764""#,
            vec![TokenKind::StringLiteral("❤".to_string())]
        );
        lex_ok!(
            r#""\uFA84""#,
            vec![TokenKind::StringLiteral("彩".to_string())]
        );
        lex_ok!(
            r#""\uD83D\uDC69""#,
            vec![TokenKind::StringLiteral("👩".to_string())]
        );
        lex_error!(
            r#""\uDCAC\u1234""#,
            LexerError::InvalidUnicode(
                1238580,
                Position {
                    full_index: 13,
                    line_number: 1,
                    line_char_index: 13
                }
            )
        );
    }
}
