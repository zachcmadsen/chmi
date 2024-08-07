use std::fmt;

use logos::Logos;

#[derive(Debug, Logos, PartialEq, Clone)]
#[logos(skip "\x00")]
pub enum Token {
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token(" ")]
    Space,

    #[token("vcp")]
    Vcp,

    // TODO: Rename this to hex number or something? It represents VCP codes
    // OR values so the name is a little misleading.
    #[regex("[0-9A-F][0-9A-F]", |lex| u8::from_str_radix(lex.slice(), 16).unwrap())]
    Code(u8),
    #[regex("[a-zA-Z0-9_]+", |lex| lex.slice().to_owned())]
    Identifier(String),
    #[regex("[0-9]\\.[0-9]")]
    Version,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match &self {
            Token::LeftParen => "'('",
            Token::RightParen => "')'",
            Token::Space => "' '",

            Token::Vcp => "vcp",

            Token::Code(_) => "VCP code",
            Token::Identifier(_) => "identifier",
            Token::Version => "version",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unexpected character(s) '{0}'")]
    InvalidToken(String),
    #[error("expected {0}, got {1}")]
    Expected(Token, Token),
    #[error("expected VCP code or value, got {0}")]
    ExpectedVcpCodeOrValue(Token),
    #[error("unexpected EOF")]
    EndOfFile,
}

#[derive(Debug)]
pub struct Command {
    pub code: u8,
    pub values: Vec<u8>,
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Parser<'a> {
        Parser { tokens, index: 0 }
    }

    fn parse_commands(&mut self) -> Result<Vec<Command>, ParseError> {
        self.eat_until(&Token::Vcp);

        self.expect(Token::Vcp)?;
        self.expect(Token::LeftParen)?;

        let mut commands = Vec::new();
        while !self.check(&Token::RightParen) {
            let command = self.parse_command()?;
            commands.push(command)
        }
        self.expect(Token::RightParen)?;

        Ok(commands)
    }

    fn parse_command(&mut self) -> Result<Command, ParseError> {
        let code = self.parse_code()?;

        let mut values = Vec::new();
        if self.eat(&Token::LeftParen) {
            while !self.check(&Token::RightParen) {
                let value = self.parse_code()?;
                values.push(value);
            }
            self.expect(Token::RightParen)?;
        };

        Ok(Command { code, values })
    }

    fn parse_code(&mut self) -> Result<u8, ParseError> {
        // Handle extra spaces before codes.
        self.eat(&Token::Space);
        let code = match self.next() {
            Ok(Token::Code(c)) => c,
            Ok(token) => {
                return Err(ParseError::ExpectedVcpCodeOrValue(token))
            }
            Err(err) => return Err(err),
        };
        self.eat(&Token::Space);
        Ok(code)
    }

    /// Returns true if the next token is `token`.
    fn check(&mut self, token: &Token) -> bool {
        self.tokens.get(self.index).is_some_and(|t| t == token)
    }

    /// Returns the next token.
    fn next(&mut self) -> Result<Token, ParseError> {
        self.tokens
            .get(self.index)
            .map(|t| {
                self.index += 1;
                t.clone()
            })
            .ok_or(ParseError::EndOfFile)
    }

    /// Consumes the next token if it's `token`, and returns whether the token
    /// was consumed.
    fn eat(&mut self, token: &Token) -> bool {
        let matches = self.check(&token);
        if matches {
            self.index += 1;
        }
        matches
    }

    /// Consumes tokens until the next token is `token`.
    fn eat_until(&mut self, token: &Token) {
        while self.index < self.tokens.len() && !self.check(&token) {
            self.index += 1;
        }
    }

    /// Consumes and expects `token`.
    fn expect(&mut self, token: Token) -> Result<(), ParseError> {
        let next = self.next()?;
        if next == token {
            Ok(())
        } else {
            Err(ParseError::Expected(next, token))
        }
    }
}

pub fn parse(s: &str) -> Result<Vec<Command>, ParseError> {
    let mut tokens = Vec::new();
    for (token, span) in Token::lexer(s).spanned() {
        match token {
            Ok(token) => tokens.push(token),
            Err(_) => {
                return Err(ParseError::InvalidToken(format!(
                    "{}",
                    &s[span.start..span.end]
                )))
            }
        }
    }

    let mut parser = Parser::new(&tokens);
    parser.parse_commands()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_token() {
        let source = "(prot($))";
        let result = parse(&source);
        assert!(matches!(result, Err(ParseError::InvalidToken(_))));
    }

    #[test]
    fn eof() {
        let source = "(prot(monitor";
        let result = parse(&source);
        assert!(matches!(result, Err(ParseError::EndOfFile)));

        let source = "(vcp(01 02";
        let result = parse(&source);
        assert!(matches!(result, Err(ParseError::EndOfFile)));
    }

    #[test]
    fn expected_vcp_code_or_value() {
        let source = "(vcp(01 02 monitor))";
        let result = parse(&source);
        assert!(matches!(result, Err(ParseError::ExpectedVcpCodeOrValue(_))));
    }

    #[test]
    fn unexpected_token() {
        let source = "(vcp)01 02 03))";
        let result = parse(&source);
        assert!(matches!(result, Err(ParseError::Expected(_, _))));
    }

    #[test]
    fn un880() {
        let source = "(prot(monitor)type(lcd)UN880cmds(01 02 03 0C E3 F3)vcp(02 04 05 08 10 12 14(05 08 0B ) 16 18 1A 52 60( 11 12 0F 00) AC AE B2 B6 C0 C6 C8 C9 D6(01 04) DF 62 8D F4 F5(00 01 02) F6(00 01 02) 4D 4E 4F 15(01 06 11 13 14 15 18 19 28 29 48) F7(00 01 02 03) F8(00 01) F9 E4 E5 E6 E7 E8 E9 EA EB EF FD(00 01) FE(00 01 02) FF)mccs_ver(2.1)mswhql(1))";
        let commands = parse(&source).unwrap();
        insta::assert_debug_snapshot!(commands, @r###"
        [
            Command {
                code: 2,
                values: [],
            },
            Command {
                code: 4,
                values: [],
            },
            Command {
                code: 5,
                values: [],
            },
            Command {
                code: 8,
                values: [],
            },
            Command {
                code: 16,
                values: [],
            },
            Command {
                code: 18,
                values: [],
            },
            Command {
                code: 20,
                values: [
                    5,
                    8,
                    11,
                ],
            },
            Command {
                code: 22,
                values: [],
            },
            Command {
                code: 24,
                values: [],
            },
            Command {
                code: 26,
                values: [],
            },
            Command {
                code: 82,
                values: [],
            },
            Command {
                code: 96,
                values: [
                    17,
                    18,
                    15,
                    0,
                ],
            },
            Command {
                code: 172,
                values: [],
            },
            Command {
                code: 174,
                values: [],
            },
            Command {
                code: 178,
                values: [],
            },
            Command {
                code: 182,
                values: [],
            },
            Command {
                code: 192,
                values: [],
            },
            Command {
                code: 198,
                values: [],
            },
            Command {
                code: 200,
                values: [],
            },
            Command {
                code: 201,
                values: [],
            },
            Command {
                code: 214,
                values: [
                    1,
                    4,
                ],
            },
            Command {
                code: 223,
                values: [],
            },
            Command {
                code: 98,
                values: [],
            },
            Command {
                code: 141,
                values: [],
            },
            Command {
                code: 244,
                values: [],
            },
            Command {
                code: 245,
                values: [
                    0,
                    1,
                    2,
                ],
            },
            Command {
                code: 246,
                values: [
                    0,
                    1,
                    2,
                ],
            },
            Command {
                code: 77,
                values: [],
            },
            Command {
                code: 78,
                values: [],
            },
            Command {
                code: 79,
                values: [],
            },
            Command {
                code: 21,
                values: [
                    1,
                    6,
                    17,
                    19,
                    20,
                    21,
                    24,
                    25,
                    40,
                    41,
                    72,
                ],
            },
            Command {
                code: 247,
                values: [
                    0,
                    1,
                    2,
                    3,
                ],
            },
            Command {
                code: 248,
                values: [
                    0,
                    1,
                ],
            },
            Command {
                code: 249,
                values: [],
            },
            Command {
                code: 228,
                values: [],
            },
            Command {
                code: 229,
                values: [],
            },
            Command {
                code: 230,
                values: [],
            },
            Command {
                code: 231,
                values: [],
            },
            Command {
                code: 232,
                values: [],
            },
            Command {
                code: 233,
                values: [],
            },
            Command {
                code: 234,
                values: [],
            },
            Command {
                code: 235,
                values: [],
            },
            Command {
                code: 239,
                values: [],
            },
            Command {
                code: 253,
                values: [
                    0,
                    1,
                ],
            },
            Command {
                code: 254,
                values: [
                    0,
                    1,
                    2,
                ],
            },
            Command {
                code: 255,
                values: [],
            },
        ]
        "###);
    }

    #[test]
    fn vg259() {
        let source = "(prot(monitor) type(LCD)model(VG259) cmds(01 02 03 07 0C F3) vcp(02 04 05 08 10 12 14(05 06 08 0B) 16 18 1A 52 60(11 12 0F) 62 6C 6E 70 86(02 0B) 87(00 0A 14 1E 28 32 3C 46 50 5A 64) 8A 8D(01 02) AC AE B6 C6 C8 C9 CC(01 02 03 04 05 06 07 08 09 0A 0C 0D 11 12 14 1A 1E 1F 23 30 31) D6(01 05) DC(01 02 03 04 05 06 07 08) DF E0(00 01 02 03 04 05) E1(00 01) E3(00 01 02 03 04 05 06) E4(00 01 02 03 04 05) E5(00 01 02 03) E6(00 01 02 03 04) E7(00 01) E9(00 01) EA(00 01) EB(00 01))mccs_ver(2.2)asset_eep(32)mpu(01)mswhql(1))";
        let commands = parse(&source).unwrap();
        insta::assert_debug_snapshot!(commands, @r###"
        [
            Command {
                code: 2,
                values: [],
            },
            Command {
                code: 4,
                values: [],
            },
            Command {
                code: 5,
                values: [],
            },
            Command {
                code: 8,
                values: [],
            },
            Command {
                code: 16,
                values: [],
            },
            Command {
                code: 18,
                values: [],
            },
            Command {
                code: 20,
                values: [
                    5,
                    6,
                    8,
                    11,
                ],
            },
            Command {
                code: 22,
                values: [],
            },
            Command {
                code: 24,
                values: [],
            },
            Command {
                code: 26,
                values: [],
            },
            Command {
                code: 82,
                values: [],
            },
            Command {
                code: 96,
                values: [
                    17,
                    18,
                    15,
                ],
            },
            Command {
                code: 98,
                values: [],
            },
            Command {
                code: 108,
                values: [],
            },
            Command {
                code: 110,
                values: [],
            },
            Command {
                code: 112,
                values: [],
            },
            Command {
                code: 134,
                values: [
                    2,
                    11,
                ],
            },
            Command {
                code: 135,
                values: [
                    0,
                    10,
                    20,
                    30,
                    40,
                    50,
                    60,
                    70,
                    80,
                    90,
                    100,
                ],
            },
            Command {
                code: 138,
                values: [],
            },
            Command {
                code: 141,
                values: [
                    1,
                    2,
                ],
            },
            Command {
                code: 172,
                values: [],
            },
            Command {
                code: 174,
                values: [],
            },
            Command {
                code: 182,
                values: [],
            },
            Command {
                code: 198,
                values: [],
            },
            Command {
                code: 200,
                values: [],
            },
            Command {
                code: 201,
                values: [],
            },
            Command {
                code: 204,
                values: [
                    1,
                    2,
                    3,
                    4,
                    5,
                    6,
                    7,
                    8,
                    9,
                    10,
                    12,
                    13,
                    17,
                    18,
                    20,
                    26,
                    30,
                    31,
                    35,
                    48,
                    49,
                ],
            },
            Command {
                code: 214,
                values: [
                    1,
                    5,
                ],
            },
            Command {
                code: 220,
                values: [
                    1,
                    2,
                    3,
                    4,
                    5,
                    6,
                    7,
                    8,
                ],
            },
            Command {
                code: 223,
                values: [],
            },
            Command {
                code: 224,
                values: [
                    0,
                    1,
                    2,
                    3,
                    4,
                    5,
                ],
            },
            Command {
                code: 225,
                values: [
                    0,
                    1,
                ],
            },
            Command {
                code: 227,
                values: [
                    0,
                    1,
                    2,
                    3,
                    4,
                    5,
                    6,
                ],
            },
            Command {
                code: 228,
                values: [
                    0,
                    1,
                    2,
                    3,
                    4,
                    5,
                ],
            },
            Command {
                code: 229,
                values: [
                    0,
                    1,
                    2,
                    3,
                ],
            },
            Command {
                code: 230,
                values: [
                    0,
                    1,
                    2,
                    3,
                    4,
                ],
            },
            Command {
                code: 231,
                values: [
                    0,
                    1,
                ],
            },
            Command {
                code: 233,
                values: [
                    0,
                    1,
                ],
            },
            Command {
                code: 234,
                values: [
                    0,
                    1,
                ],
            },
            Command {
                code: 235,
                values: [
                    0,
                    1,
                ],
            },
        ]
        "###);
    }

    #[test]
    fn samsung() {
        let source = "(prot(monitor)type(lcd)SAMSUNGcmds(01 02 03 07 0C E3 F3)vcp(02 04 05 08 10 12 14(05 08 0B 0C) 16 18 1A 52 60( 11 12 0F) AC AE B2 B6 C6 C8 C9 D6(01 04 05) DC(00 02 03 05 ) DF FD)mccs_ver(2.1)mswhql(1))";
        let commands = parse(&source).unwrap();
        insta::assert_debug_snapshot!(commands, @r###"
        [
            Command {
                code: 2,
                values: [],
            },
            Command {
                code: 4,
                values: [],
            },
            Command {
                code: 5,
                values: [],
            },
            Command {
                code: 8,
                values: [],
            },
            Command {
                code: 16,
                values: [],
            },
            Command {
                code: 18,
                values: [],
            },
            Command {
                code: 20,
                values: [
                    5,
                    8,
                    11,
                    12,
                ],
            },
            Command {
                code: 22,
                values: [],
            },
            Command {
                code: 24,
                values: [],
            },
            Command {
                code: 26,
                values: [],
            },
            Command {
                code: 82,
                values: [],
            },
            Command {
                code: 96,
                values: [
                    17,
                    18,
                    15,
                ],
            },
            Command {
                code: 172,
                values: [],
            },
            Command {
                code: 174,
                values: [],
            },
            Command {
                code: 178,
                values: [],
            },
            Command {
                code: 182,
                values: [],
            },
            Command {
                code: 198,
                values: [],
            },
            Command {
                code: 200,
                values: [],
            },
            Command {
                code: 201,
                values: [],
            },
            Command {
                code: 214,
                values: [
                    1,
                    4,
                    5,
                ],
            },
            Command {
                code: 220,
                values: [
                    0,
                    2,
                    3,
                    5,
                ],
            },
            Command {
                code: 223,
                values: [],
            },
            Command {
                code: 253,
                values: [],
            },
        ]
        "###);
    }
}
