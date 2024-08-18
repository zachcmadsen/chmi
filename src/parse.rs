use std::fmt;

use anyhow::{anyhow, Context};
use logos::Logos;

use crate::cap::{Capabilities, VcpCode};

#[derive(Clone, Copy, Debug, Logos, PartialEq)]
#[logos(skip "[ \x00]")]
enum Token {
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
            Token::Vcp => "'vcp'",
            Token::HexNumber(_) => "hexadecimal number",
            Token::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

struct CapabilitiesStringParser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> CapabilitiesStringParser<'a> {
    fn new(tokens: &'a [Token]) -> CapabilitiesStringParser<'a> {
        CapabilitiesStringParser { tokens, index: 0 }
    }

    fn parse(&mut self) -> anyhow::Result<Capabilities> {
        let mut capabilities = Capabilities { vcp: None };

        self.expect(Token::LeftParen)?;
        while !self.check(Token::RightParen) {
            match self.next()? {
                Token::Vcp => capabilities.vcp = Some(self.parse_vcp()?),
                Token::Unknown => {
                    self.expect(Token::LeftParen)?;
                    self.eat_until(Token::RightParen);
                    self.expect(Token::RightParen)?;
                }
                _ => panic!("invalid capabilities string"),
            };
        }
        self.expect(Token::RightParen)?;

        Ok(capabilities)
    }

    fn parse_vcp(&mut self) -> anyhow::Result<Vec<VcpCode>> {
        self.expect(Token::LeftParen)?;
        let mut vcp_codes = Vec::new();
        while !self.check(Token::RightParen) {
            let vcp_code = self.parse_vcp_code()?;
            vcp_codes.push(vcp_code)
        }
        self.expect(Token::RightParen)?;
        Ok(vcp_codes)
    }

    fn parse_vcp_code(&mut self) -> anyhow::Result<VcpCode> {
        let code = self.parse_number()?;
        let mut values = Vec::new();
        if self.eat(Token::LeftParen) {
            while !self.check(Token::RightParen) {
                let value = self.parse_number()?;
                values.push(value);
            }
            self.expect(Token::RightParen)?;
        };
        Ok(VcpCode { code, values })
    }

    fn parse_number(&mut self) -> anyhow::Result<u8> {
        match self.next()? {
            Token::HexNumber(n) => Ok(n),
            token => {
                return Err(anyhow!(
                    "expected hexadecimal number, found {}",
                    token
                ))
            }
        }
    }

    /// Consumes and expects `token`.
    fn expect(&mut self, token: Token) -> anyhow::Result<()> {
        let t = self.next()?;
        if t == token {
            Ok(())
        } else {
            Err(anyhow!("expected {}, found {}", token, t))
        }
    }

    /// Consumes the next token if it's `token`, and returns whether the token
    /// was consumed.
    fn eat(&mut self, token: Token) -> bool {
        self.check(token)
            .then(|| {
                self.index += 1;
            })
            .is_some()
    }

    /// Consumes tokens until the next token is `token`.
    fn eat_until(&mut self, token: Token) {
        while self.index < self.tokens.len() && !self.check(token) {
            self.index += 1;
        }
    }

    /// Returns true if the next token is `token`.
    fn check(&self, token: Token) -> bool {
        self.tokens.get(self.index).is_some_and(|&t| t == token)
    }

    /// Returns the next token.
    fn next(&mut self) -> anyhow::Result<Token> {
        self.tokens
            .get(self.index)
            .map(|t| {
                self.index += 1;
                t.clone()
            })
            .ok_or(anyhow!("unexpected end-of-file"))
    }
}

pub fn parse(capabilities_string: &str) -> anyhow::Result<Capabilities> {
    let mut tokens = Vec::new();
    for (token, span) in Token::lexer(capabilities_string).spanned() {
        match token {
            Ok(token) => tokens.push(token),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "unexpected character(s) '{}'",
                    &capabilities_string[span.start..span.end],
                ))
            }
        }
    }

    CapabilitiesStringParser::new(&tokens)
        .parse()
        .context("failed to parse capabilities string")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_un880_capabilities() {
        let capabilities_string = "(prot(monitor)type(lcd)UN880cmds(01 02 03 0C E3 F3)vcp(02 04 05 08 10 12 14(05 08 0B ) 16 18 1A 52 60( 11 12 0F 00) AC AE B2 B6 C0 C6 C8 C9 D6(01 04) DF 62 8D F4 F5(00 01 02) F6(00 01 02) 4D 4E 4F 15(01 06 11 13 14 15 18 19 28 29 48) F7(00 01 02 03) F8(00 01) F9 E4 E5 E6 E7 E8 E9 EA EB EF FD(00 01) FE(00 01 02) FF)mccs_ver(2.1)mswhql(1))";
        let capabilities = parse(&capabilities_string).unwrap();
        insta::assert_debug_snapshot!(capabilities);
    }

    #[test]
    fn parse_u32j59x_capabilities() {
        let capabilities_string = "(prot(monitor)type(lcd)SAMSUNGcmds(01 02 03 07 0C E3 F3)vcp(02 04 05 08 10 12 14(05 08 0B 0C) 16 18 1A 52 60( 11 12 0F) AC AE B2 B6 C6 C8 C9 D6(01 04 05) DC(00 02 03 05 ) DF FD)mccs_ver(2.1)mswhql(1))";
        let capabilities = parse(&capabilities_string).unwrap();
        insta::assert_debug_snapshot!(capabilities);
    }

    #[test]
    fn parse_vg259_capabilities() {
        let capabilities_string = "(prot(monitor) type(LCD)model(VG259) cmds(01 02 03 07 0C F3) vcp(02 04 05 08 10 12 14(05 06 08 0B) 16 18 1A 52 60(11 12 0F) 62 6C 6E 70 86(02 0B) 87(00 0A 14 1E 28 32 3C 46 50 5A 64) 8A 8D(01 02) AC AE B6 C6 C8 C9 CC(01 02 03 04 05 06 07 08 09 0A 0C 0D 11 12 14 1A 1E 1F 23 30 31) D6(01 05) DC(01 02 03 04 05 06 07 08) DF E0(00 01 02 03 04 05) E1(00 01) E3(00 01 02 03 04 05 06) E4(00 01 02 03 04 05) E5(00 01 02 03) E6(00 01 02 03 04) E7(00 01) E9(00 01) EA(00 01) EB(00 01))mccs_ver(2.2)asset_eep(32)mpu(01)mswhql(1))";
        let capabilities = parse(&capabilities_string).unwrap();
        insta::assert_debug_snapshot!(capabilities);
    }
}
