//! Expression grammar and lexer behavior from ST4 4.3.3 STParser.g/STLexer.java.
// Copyright (c) 2011 Terence Parr. All rights reserved.
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
// 1. Redistributions of source code must retain the above copyright notice,
//    this list of conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
// 3. The name of the author may not be used to endorse or promote products
//    derived from this software without specific prior written permission.
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR IMPLIED
// WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
// IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
// OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
// OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
// ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Id,
    String,
    Anonymous,
    Symbol,
}
struct Lexeme {
    kind: Kind,
    text: String,
}

fn failure() -> String {
    format!("{COMPILE_ERROR}invalid expression syntax")
}
fn id(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}
fn ws(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\r' | '\n')
}

pub(super) struct Anonymous<'a> {
    pub parameters: Vec<String>,
    pub body: &'a str,
    pub initial_line_start: bool,
}

pub(super) fn anonymous(input: &str) -> Result<Anonymous<'_>, String> {
    let inner = input
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .ok_or_else(failure)?;
    let skip = |mut p: usize| {
        while p < inner.len() && ws(inner[p..].chars().next().unwrap()) {
            p += inner[p..].chars().next().unwrap().len_utf8();
        }
        p
    };
    let mut p = skip(0);
    let mut parameters = Vec::new();
    loop {
        let start = p;
        if p == inner.len() {
            break;
        }
        // subTemplate's speculative mID consumes its first character directly.
        let Some(width) = utf16::byte_index(&inner[p..], 1) else {
            // A supplementary first character leaves a low surrogate in the
            // speculative ID scan, so the following pipe is not recognized.
            return Ok(Anonymous {
                parameters: Vec::new(),
                body: inner,
                initial_line_start: false,
            });
        };
        p += width;
        while p < inner.len() && id(inner[p..].chars().next().unwrap()) {
            p += 1;
        }
        parameters.push(inner[start..p].to_owned());
        p = skip(p);
        if inner[p..].starts_with(',') {
            p = skip(p + 1);
        } else {
            break;
        }
    }
    if inner[p..].starts_with('|') {
        let body = &inner[p + 1..];
        let initial_line_start = body.starts_with('\n');
        let body = body
            .chars()
            .next()
            .filter(|c| ws(*c))
            .map(|c| &body[c.len_utf8()..])
            .unwrap_or(body);
        Ok(Anonymous {
            parameters,
            body,
            initial_line_start,
        })
    } else {
        Ok(Anonymous {
            parameters: Vec::new(),
            body: inner,
            initial_line_start: false,
        })
    }
}

fn lex(input: &str, delimiters: delimiters::Delimiters) -> Result<Vec<Lexeme>, String> {
    lex_units(&input.encode_utf16().collect::<Vec<_>>(), delimiters)
}

fn lex_units(input: &[u16], delimiters: delimiters::Delimiters) -> Result<Vec<Lexeme>, String> {
    let mut tokens = Vec::new();
    let (mut p, mut mark) = (0, 0);
    while p < input.len() {
        let ch = input[p];
        let start = p;
        p += 1;
        if matches!(ch, 9 | 10 | 13 | 32) {
            mark = p;
            continue;
        }
        let mut kind = Kind::Symbol;
        let text = if ch == 34 {
            kind = Kind::String;
            let mut text = vec![34];
            let (mut closed, mut escaped) = (false, false);
            while p < input.len() {
                let c = input[p];
                p += 1;
                if c == 34 {
                    closed = true;
                    break;
                }
                if c == 92 {
                    escaped = true;
                    let c = *input.get(p).ok_or_else(failure)?;
                    p += 1;
                    text.push(match c {
                        110 => 10,
                        114 => 13,
                        116 => 9,
                        c => c,
                    });
                } else {
                    text.push(c);
                }
            }
            if !closed {
                return Err(failure());
            }
            text.push(34);
            // inside() does not reset startCharIndex when skipping an invalid
            // prefix. mSTRING preserves that prefix unless it saw an escape.
            let raw = if escaped { &text[..] } else { &input[mark..p] };
            Text(raw[1..raw.len() - 1].to_vec()).json_literal()
        } else if ch == 123 {
            kind = Kind::Anonymous;
            p = utf16::anonymous_end(input, start, delimiters).map_err(|_| failure())?;
            let text = Text(input[start..p].to_vec())
                .valid_string()
                .ok_or_else(failure)?;
            let value = anonymous(&text)?;
            tokenize_at(value.body, true, value.initial_line_start, delimiters)?;
            text
        } else if char::from_u32(u32::from(ch)).is_some_and(id) {
            while p < input.len() && char::from_u32(u32::from(input[p])).is_some_and(id) {
                p += 1;
            }
            let text = Text(input[start..p].to_vec()).valid_string().unwrap();
            if !matches!(
                text.as_str(),
                "if" | "elseif" | "else" | "endif" | "super" | "true" | "false"
            ) {
                kind = Kind::Id;
            }
            text
        } else if ch == 46 && input[p..].starts_with(&[46, 46]) {
            p += 2;
            "...".into()
        } else if ch == 64 && input[p..].starts_with(&[101, 110, 100]) {
            p += 3;
            "@end".into()
        } else if matches!(ch, 38 | 124) {
            p = (p + 1).min(input.len());
            if ch == 38 { "&&".into() } else { "||".into() }
        } else if matches!(
            ch,
            46 | 44 | 58 | 59 | 40 | 41 | 91 | 93 | 61 | 33 | 47 | 64
        ) {
            char::from_u32(u32::from(ch)).unwrap().to_string()
        } else {
            continue;
        };
        tokens.push(Lexeme { kind, text });
        mark = p;
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Lexeme>,
    position: usize,
    conditional: usize,
}
impl Parser {
    fn text(&self) -> &str {
        self.tokens
            .get(self.position)
            .map(|t| t.text.as_str())
            .unwrap_or("")
    }
    fn at(&self, n: usize) -> &str {
        self.tokens
            .get(self.position + n)
            .map(|t| t.text.as_str())
            .unwrap_or("")
    }
    fn is_id(&self) -> bool {
        self.tokens
            .get(self.position)
            .is_some_and(|t| t.kind == Kind::Id)
    }
    fn take(&mut self, text: &str) -> bool {
        if self.text() == text {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn require(&mut self, text: &str) -> Result<(), String> {
        if self.take(text) {
            Ok(())
        } else {
            Err(failure())
        }
    }
    fn identifier(&mut self) -> Result<(), String> {
        if self.is_id() {
            self.position += 1;
            Ok(())
        } else {
            Err(failure())
        }
    }
    fn rendered(&self, start: usize, end: usize) -> String {
        self.tokens[start..end]
            .iter()
            .map(|t| t.text.as_str())
            .collect()
    }
    fn finished(&self) -> Result<(), String> {
        if self.position == self.tokens.len() {
            Ok(())
        } else {
            Err(failure())
        }
    }

    fn conditional(&mut self) -> Result<(), String> {
        self.and_condition()?;
        while self.take("||") {
            self.and_condition()?;
        }
        Ok(())
    }
    fn and_condition(&mut self) -> Result<(), String> {
        self.not_condition()?;
        while self.take("&&") {
            self.not_condition()?;
        }
        Ok(())
    }
    fn not_condition(&mut self) -> Result<(), String> {
        if self.take("!") {
            self.not_condition()
        } else {
            self.member()
        }
    }
    fn map_expression(&mut self) -> Result<(), String> {
        self.member()?;
        let zipped = self.text() == ",";
        if zipped {
            while self.take(",") {
                self.member()?;
            }
            self.require(":")?;
            self.prototype()?;
        }
        while self.take(":") {
            self.prototype()?;
            if !zipped {
                while self.take(",") {
                    self.prototype()?;
                }
            }
        }
        Ok(())
    }
    fn no_comma(&mut self) -> Result<(), String> {
        self.member()?;
        if self.take(":") {
            self.prototype()?;
        }
        Ok(())
    }
    fn qualified(&mut self) -> Result<(), String> {
        self.take("/");
        self.identifier()?;
        while self.take("/") {
            self.identifier()?;
        }
        Ok(())
    }
    fn is_qualified_call(&self) -> bool {
        let mut p = self.position;
        if self.tokens.get(p).is_some_and(|t| t.text == "/") {
            p += 1;
        }
        if !self.tokens.get(p).is_some_and(|t| t.kind == Kind::Id) {
            return false;
        }
        p += 1;
        while self.tokens.get(p).is_some_and(|t| t.text == "/") {
            p += 1;
            if !self.tokens.get(p).is_some_and(|t| t.kind == Kind::Id) {
                return false;
            }
            p += 1;
        }
        self.tokens.get(p).is_some_and(|t| t.text == "(")
    }
    fn member(&mut self) -> Result<(), String> {
        if self.is_id() && expression::builtin(self.text()) && self.at(1) == "(" {
            self.position += 2;
            if self.text() != ")" {
                self.map_expression()?;
            }
            self.require(")")?;
        } else if self.take("super") {
            self.require(".")?;
            self.identifier()?;
            self.require("(")?;
            self.arguments()?;
            self.require(")")?;
        } else if self.is_qualified_call() {
            self.qualified()?;
            self.require("(")?;
            self.arguments()?;
            self.require(")")?;
        } else if self.take("@") {
            if self.take("super") {
                self.require(".")?;
            }
            self.identifier()?;
            self.require("(")?;
            self.require(")")?;
        } else {
            self.primary()?;
        }
        while self.take(".") {
            if self.take("(") {
                self.map_expression()?;
                self.require(")")?;
            } else {
                self.identifier()?;
            }
        }
        Ok(())
    }
    fn primary(&mut self) -> Result<(), String> {
        if self.is_id()
            || self
                .tokens
                .get(self.position)
                .is_some_and(|t| matches!(t.kind, Kind::String | Kind::Anonymous))
            || matches!(self.text(), "true" | "false")
        {
            self.position += 1;
            return Ok(());
        }
        if self.take("[") {
            if self.take("]") {
                return Ok(());
            }
            loop {
                if !matches!(self.text(), "," | "]") {
                    self.no_comma()?;
                }
                if self.take("]") {
                    break;
                }
                self.require(",")?;
            }
            return Ok(());
        }
        if self.take("(") {
            if self.conditional > 0 {
                self.conditional()?;
                self.require(")")?;
            } else {
                self.map_expression()?;
                self.require(")")?;
                if self.take("(") {
                    self.positional()?;
                    self.require(")")?;
                }
            }
            return Ok(());
        }
        Err(failure())
    }
    fn prototype(&mut self) -> Result<(), String> {
        if self
            .tokens
            .get(self.position)
            .is_some_and(|t| t.kind == Kind::Anonymous)
        {
            self.position += 1;
            return Ok(());
        }
        if self.take("(") {
            self.map_expression()?;
            self.require(")")?;
            self.require("(")?;
            self.positional()?;
            self.require(")")?;
            return Ok(());
        }
        self.qualified()?;
        self.require("(")?;
        self.arguments()?;
        self.require(")")
    }
    fn positional(&mut self) -> Result<(), String> {
        if self.text() == ")" {
            return Ok(());
        }
        self.no_comma()?;
        while self.take(",") {
            self.no_comma()?;
        }
        Ok(())
    }
    fn arguments(&mut self) -> Result<(), String> {
        if self.text() == ")" || self.take("...") {
            return Ok(());
        }
        if self.is_id() && self.at(1) == "=" {
            loop {
                self.identifier()?;
                self.require("=")?;
                self.no_comma()?;
                if !self.take(",") {
                    break;
                }
                if self.take("...") {
                    break;
                }
            }
            Ok(())
        } else {
            self.positional()
        }
    }
    fn tag(mut self) -> Result<String, String> {
        if matches!(self.text(), "if" | "elseif") {
            self.position += 1;
            self.require("(")?;
            self.conditional += 1;
            self.conditional()?;
            self.conditional -= 1;
            self.require(")")?;
            self.finished()?;
            return Ok(self.rendered(0, self.position));
        }
        if matches!(self.text(), "else" | "endif" | "@end") {
            self.position += 1;
            self.finished()?;
            return Ok(self.rendered(0, self.position));
        }
        if self.text() == "@"
            && self.tokens.get(1).is_some_and(|t| t.kind == Kind::Id)
            && self.tokens.len() == 2
        {
            return Ok(self.rendered(0, 2));
        }
        self.map_expression()?;
        let expression = self.rendered(0, self.position);
        let mut options = Vec::new();
        if self.take(";") {
            loop {
                let start = self.position;
                let name = self.text().to_owned();
                self.identifier()?;
                let assigned = self.take("=");
                if assigned {
                    self.no_comma()?;
                }
                if matches!(
                    name.as_str(),
                    "anchor" | "format" | "null" | "separator" | "wrap"
                ) {
                    if assigned {
                        options.push(self.rendered(start, self.position));
                    } else if matches!(name.as_str(), "anchor" | "wrap") {
                        options.push(format!(
                            "{}={}",
                            name,
                            serde_json::to_string(if name == "anchor" { "true" } else { "\n" })
                                .unwrap()
                        ));
                    }
                }
                if !self.take(",") {
                    break;
                }
            }
        }
        self.finished()?;
        Ok(if options.is_empty() {
            expression
        } else {
            format!("{};{}", expression, options.join(","))
        })
    }
}

pub(super) fn tag_units(
    input: &[u16],
    delimiters: delimiters::Delimiters,
) -> Result<String, String> {
    Parser {
        tokens: lex_units(input, delimiters)?,
        position: 0,
        conditional: 0,
    }
    .tag()
}

pub(super) fn tag(input: &str, delimiters: delimiters::Delimiters) -> Result<String, String> {
    Parser {
        tokens: lex(input, delimiters)?,
        position: 0,
        conditional: 0,
    }
    .tag()
}

pub(super) fn structure(tokens: &[Token]) -> Result<(), String> {
    enum Frame {
        If(bool),
        Region,
    }
    let mut stack = Vec::new();
    for token in tokens {
        let Token::Tag { expr, .. } = token else {
            continue;
        };
        if expr.starts_with("if(") {
            stack.push(Frame::If(false));
        } else if expr.starts_with("elseif(") {
            if !matches!(stack.last(), Some(Frame::If(false))) {
                return Err(failure());
            }
        } else if expr == "else" {
            let Some(Frame::If(seen)) = stack.last_mut() else {
                return Err(failure());
            };
            if *seen {
                return Err(failure());
            }
            *seen = true;
        } else if expr == "endif" {
            if !matches!(stack.pop(), Some(Frame::If(_))) {
                return Err(failure());
            }
        } else if expr == "@end" {
            if !matches!(stack.pop(), Some(Frame::Region)) {
                return Err(failure());
            }
        } else if expr.starts_with('@') && !expr.contains('(') {
            stack.push(Frame::Region);
        }
    }
    if stack.is_empty() {
        Ok(())
    } else {
        Err(failure())
    }
}
