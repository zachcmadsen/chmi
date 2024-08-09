use std::fmt;

use logos::Logos;

#[derive(Clone, Copy, Debug, Logos, PartialEq)]
#[logos(skip "[ \x00]")]
pub enum Token {
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,

    #[token("vcp")]
    Vcp,
    #[regex("[0-9A-F][0-9A-F]", |lex| u8::from_str_radix(lex.slice(), 16).unwrap())]
    HexNumber(u8),

    #[regex("[a-zA-Z0-9_\\.]+")]
    Unknown,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match &self {
            Token::LeftParen => "'('",
            Token::RightParen => "')'",
            Token::Vcp => "vcp",
            Token::HexNumber(_) => "hexadecimal number",
            Token::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}
