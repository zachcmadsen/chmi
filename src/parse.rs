use anyhow::anyhow;
use logos::Logos;

use crate::{
    cap::{Capabilities, VcpCode},
    token::Token,
};

pub fn parse_capabilities_string(
    capabilities_string: &str,
) -> anyhow::Result<Capabilities> {
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

    Parser::new(&tokens).parse()
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Parser<'a> {
        Parser { tokens, index: 0 }
    }

    fn parse(&mut self) -> anyhow::Result<Capabilities> {
        let mut capabilites = Capabilities { vcp: None };

        self.expect(Token::LeftParen)?;
        while !self.check(Token::RightParen) {
            match self.next()? {
                Token::Vcp => capabilites.vcp = Some(self.parse_vcp()?),
                Token::Unknown => {
                    // TODO: Skipping a capability this way breaks if it has a
                    // right paren in its value, e.g., vcp. Look ahead to see if
                    // there's a matching right paren?
                    self.expect(Token::LeftParen)?;
                    self.eat_until(Token::RightParen);
                    self.expect(Token::RightParen)?;
                }
                _ => panic!("invalid capabilities string"),
            };
        }
        self.expect(Token::RightParen)?;

        Ok(capabilites)
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
    fn check(&mut self, token: Token) -> bool {
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
