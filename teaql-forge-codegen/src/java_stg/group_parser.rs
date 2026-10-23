//! Group.g rules and ANTLR 3.5.2 token insertion/deletion and recovery.
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
use declarations::{Alias, Dictionary, Group, Header};

const EOF: i16 = -1;
const ANON: i16 = 4;
const BIG: i16 = 5;
const COMPACT: i16 = 6;
const FALSE: i16 = 8;
const ID: i16 = 9;
const LB: i16 = 10;
const RB: i16 = 12;
const STRING: i16 = 13;
const TRUE: i16 = 14;
fn bit(kind: i16) -> u64 {
    if kind < 0 { 0 } else { 1u64 << kind }
}
#[derive(Clone)]
struct Token {
    kind: i16,
    start: usize,
    end: usize,
    text: String,
    synthetic: bool,
}
struct Parser<'a> {
    source: &'a str,
    cursor: usize,
    tokens: Vec<Token>,
    position: usize,
    follows: Vec<u64>,
    last_error: Option<usize>,
    fatal: bool,
    group: Group,
}
impl<'a> Parser<'a> {
    fn lex(&mut self) -> Token {
        loop {
            let start = self.cursor;
            if start == self.source.len() {
                return Token {
                    kind: EOF,
                    start,
                    end: start,
                    text: "<EOF>".into(),
                    synthetic: false,
                };
            }
            let tail = &self.source[start..];
            let c = tail.as_bytes()[0];
            if matches!(c, b' ' | b'\t' | b'\r' | b'\n') {
                self.cursor += 1;
                continue;
            }
            if let Some(end) = dictionary::comment_end(self.source, start) {
                self.cursor = end;
                continue;
            }
            let (kind, end) = if tail.starts_with('"')
                || tail.starts_with("<<")
                || tail.starts_with("<%")
                || tail.starts_with('{')
            {
                let kind = if c == b'"' {
                    STRING
                } else if c == b'{' {
                    ANON
                } else if tail.starts_with("<%") {
                    COMPACT
                } else {
                    BIG
                };
                match dictionary::value_end_with(self.source, start, self.group.delimiters) {
                    Ok(Some(end)) => (kind, end),
                    _ => {
                        self.cursor = self.source.len();
                        continue;
                    }
                }
            } else if c.is_ascii_alphabetic() || c == b'_' {
                let mut end = start + 1;
                while end < self.source.len()
                    && (self.source.as_bytes()[end].is_ascii_alphanumeric()
                        || matches!(self.source.as_bytes()[end], b'_' | b'-'))
                {
                    end += 1;
                }
                let kind = match &self.source[start..end] {
                    "false" => FALSE,
                    "true" => TRUE,
                    "default" => 25,
                    "delimiters" => 26,
                    "group" => 27,
                    "implements" => 28,
                    "import" => 29,
                    _ => ID,
                };
                (kind, end)
            } else if tail.starts_with("::=") {
                (21, start + 3)
            } else {
                let kind = match c {
                    b'[' => LB,
                    b']' => RB,
                    b'(' => 16,
                    b')' => 17,
                    b',' => 18,
                    b'.' => 19,
                    b':' => 20,
                    b';' => 22,
                    b'=' => 23,
                    b'@' => 24,
                    _ => {
                        self.cursor += tail.chars().next().unwrap().len_utf8();
                        continue;
                    }
                };
                (kind, start + 1)
            };
            self.cursor = end;
            if matches!(kind, ANON | BIG | COMPACT | STRING) {
                self.group.ranges.push((start, end));
            }
            let text = self.source[start..end].replace("\\\"", "\"");
            return Token {
                kind,
                start,
                end,
                text,
                synthetic: false,
            };
        }
    }
    fn look(&mut self, n: usize) -> Token {
        while self.tokens.len() <= self.position + n {
            let token = self.lex();
            self.tokens.push(token);
        }
        self.tokens[self.position + n].clone()
    }
    fn at(&mut self) -> i16 {
        self.look(0).kind
    }
    fn consume(&mut self) -> Token {
        let token = self.look(0);
        if token.kind != EOF {
            self.position += 1;
        }
        token
    }
    fn contextual(&self) -> u64 {
        let mut set = 0;
        for (i, follow) in self.follows.iter().enumerate().rev() {
            set |= follow;
            if follow & 2 == 0 {
                break;
            }
            if i > 0 {
                set &= !2;
            }
        }
        set
    }
    fn take(&mut self, kind: i16, mut follow: u64) -> Result<Token, ()> {
        if self.at() == kind {
            return Ok(self.consume());
        }
        if self.look(1).kind == kind {
            self.consume();
            return Ok(self.consume());
        }
        if follow & 2 != 0 {
            follow |= self.contextual();
            if !self.follows.is_empty() {
                follow &= !2;
            }
        }
        if follow & bit(self.at()) != 0 || follow & 2 != 0 {
            let current = self.look(0);
            let name = match kind {
                ID => "ID",
                STRING => "STRING",
                LB => "'['",
                RB => "']'",
                _ => "token",
            };
            return Ok(Token {
                kind,
                start: current.start,
                end: current.start,
                text: format!("<missing {name}>"),
                synthetic: true,
            });
        }
        Err(())
    }
    fn recover(&mut self) {
        if self.last_error == Some(self.position) {
            self.consume();
        }
        self.last_error = Some(self.position);
        let set = self.follows.iter().fold(0, |set, follow| set | follow);
        while self.at() != EOF && set & bit(self.at()) == 0 {
            self.consume();
        }
    }
    fn rule<T>(&mut self, follow: u64, f: impl FnOnce(&mut Self) -> T) -> T {
        self.follows.push(follow);
        let result = f(self);
        self.follows.pop();
        result
    }
    fn old_header(&mut self) {
        let result = (|| {
            self.take(27, 0x200)?;
            self.take(ID, 0x10500000)?;
            if self.at() == 20 {
                self.take(20, 0x200)?;
                self.take(ID, 0x10400000)?;
            }
            if self.at() == 28 {
                self.take(28, 0x200)?;
                self.take(ID, 0x440000)?;
                while self.at() == 18 {
                    self.take(18, 0x200)?;
                    self.take(ID, 0x440000)?;
                }
            }
            self.take(22, 2)?;
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn delimiters(&mut self) {
        let result = (|| {
            self.take(26, 0x2000)?;
            let start = self.take(STRING, 0x40000)?;
            self.take(18, 0x2000)?;
            let stop = self.take(STRING, 2)?;
            // These are already GroupLexer STRING tokens. Re-lexing would
            // remove an escaped quote a second time (notably for '\\').
            if let Some(delimiters) = delimiters::Delimiters::from_tokens(&start.text, &stop.text) {
                self.group.delimiters = delimiters;
            }
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn parse(&mut self) {
        let result = (|| {
            if self.at() == 27 {
                self.rule(0x25000200, Self::old_header);
            }
            if self.at() == 26 {
                self.rule(0x21000200, Self::delimiters);
            }
            while self.at() == 29 {
                match self.look(1).kind {
                    STRING => {
                        self.take(29, 0x2000)?;
                        self.take(STRING, 0x21000200)?;
                    }
                    ID => {
                        self.take(29, 0x200)?;
                        self.take(ID, 0x21080200)?;
                        while self.at() == 19 {
                            self.take(19, 0x200)?;
                            self.take(ID, 0x21080200)?;
                        }
                    }
                    _ => break,
                }
            }
            while !self.fatal && matches!(self.at(), ID | 24) {
                self.rule(0x1000200, Self::definition);
            }
            self.take(EOF, 2)?;
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn definition(&mut self) {
        let result = match (self.at(), self.look(1).kind, self.look(2).kind) {
            (24, _, _) | (ID, 16, _) | (ID, 21, ID) => {
                self.rule(2, Self::template);
                Ok(())
            }
            (ID, 21, LB) => {
                self.rule(2, Self::dictionary);
                Ok(())
            }
            _ => Err(()),
        };
        if result.is_err() {
            self.last_error = Some(self.position);
            self.recover();
        }
    }
    fn template(&mut self) {
        let result = (|| {
            let position = self.look(0).start;
            if self.at() == ID && self.look(1).kind == 21 {
                let alias = self.take(ID, 0x200000)?;
                self.take(21, 0x200)?;
                let target = self.take(ID, 2)?;
                self.group.aliases.push(Alias {
                    position,
                    name: alias.text,
                    target: target.text,
                });
                return Ok(());
            }
            let (name, args) = if self.at() == 24 {
                self.take(24, 0x200)?;
                let enclosing = self.take(ID, 0x80000)?;
                self.take(19, 0x200)?;
                let name = self.take(ID, 0x10000)?;
                self.take(16, 0x20000)?;
                self.take(17, 0x200000)?;
                (
                    Token {
                        text: format!("@{}.{}", enclosing.text, name.text),
                        ..name
                    },
                    Vec::new(),
                )
            } else if self.at() == ID {
                let name = self.take(ID, 0x10000)?;
                self.take(16, 0x20200)?;
                let args = self.rule(0x20000, Self::formals);
                self.take(17, 0x200000)?;
                (name, args)
            } else {
                return Err(());
            };
            self.take(21, 0x2062)?;
            let body = match self.at() {
                STRING | BIG | COMPACT => Some(self.consume()),
                EOF | ID | 24 => None,
                _ => return Err(()),
            };
            if !name.synthetic {
                let token = body.clone().unwrap_or_else(|| self.look(0));
                self.group.headers.push(Header {
                    position,
                    name: name.text,
                    arguments: String::new(),
                    parsed_args: Some(args),
                    missing_body: body.is_none(),
                    opening: token.start,
                    end: if body.is_some() {
                        token.end
                    } else {
                        token.start
                    },
                });
            }
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn formals(&mut self) -> Vec<FormalArgument> {
        let mut args = Vec::new();
        let result = (|| {
            match self.at() {
                ID => {
                    self.rule(0x40002, |p| p.formal(&mut args));
                    while self.at() == 18 {
                        self.take(18, 0x200)?;
                        self.rule(0x40002, |p| p.formal(&mut args));
                    }
                }
                17 => {}
                _ => return Err(()),
            }
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
        args
    }
    fn formal(&mut self, args: &mut Vec<FormalArgument>) {
        let result = (|| {
            let name = self.take(ID, 0x800002)?;
            let default = if self.at() == 23 {
                match self.look(1).kind {
                    STRING | ANON | FALSE | TRUE => {
                        self.take(23, 0x6110)?;
                        let value = self.consume();
                        Some(self.source[value.start..value.end].to_owned())
                    }
                    LB => {
                        self.take(23, 0x400)?;
                        self.take(LB, 0x1000)?;
                        self.take(RB, 2)?;
                        Some("[]".into())
                    }
                    _ => return Err(()),
                }
            } else if matches!(self.at(), 17 | 18) {
                None
            } else {
                return Err(());
            };
            if !args.iter().any(|arg| arg.name == name.text) {
                args.push(FormalArgument {
                    name: name.text,
                    default,
                    default_state: DefaultState::Ready,
                });
            }
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn dictionary(&mut self) {
        let result = (|| {
            let name = self.take(ID, 0x200000)?;
            self.take(21, 0x400)?;
            let opening = self.look(0).start;
            let entries = self.rule(2, Self::dictionary_body);
            if !self.fatal {
                let end = self.look(0).start;
                self.group.dictionaries.push(Dictionary {
                    position: name.start,
                    name: name.text,
                    opening,
                    end,
                    parsed_entries: Some(entries),
                });
            }
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn dictionary_body(&mut self) -> Vec<(String, Option<String>)> {
        let mut entries = Vec::new();
        let result = (|| {
            self.take(LB, 0x2002000)?;
            self.rule(0x1000, |p| p.pairs(&mut entries));
            self.take(RB, 2)?;
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
        entries
    }
    fn pairs(&mut self, entries: &mut Vec<(String, Option<String>)>) {
        let result = (|| {
            if self.at() == STRING {
                self.rule(0x40002, |p| p.pair(entries));
                while self.at() == 18 && self.look(1).kind == STRING {
                    self.take(18, 0x2000)?;
                    self.rule(0x40002, |p| p.pair(entries));
                }
                if self.at() == 18 {
                    self.take(18, 0x2000000)?;
                    self.rule(2, |p| p.default_pair(entries));
                }
            } else if self.at() == 25 {
                self.rule(2, |p| p.default_pair(entries));
            } else {
                return Err(());
            }
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn pair(&mut self, entries: &mut Vec<(String, Option<String>)>) {
        let result = (|| {
            let key = self.take(STRING, 0x100000)?;
            self.take(20, 0x6770)?;
            let value = self.rule(2, Self::value);
            let key = &key.text[1..key.text.len() - 1];
            entries.push((
                key.replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t"),
                value,
            ));
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn default_pair(&mut self, entries: &mut Vec<(String, Option<String>)>) {
        let result = (|| {
            self.take(25, 0x100000)?;
            self.take(20, 0x6770)?;
            let value = self.rule(2, Self::value);
            entries.push(("default".into(), value));
            Ok::<_, ()>(())
        })();
        if result.is_err() {
            self.recover();
        }
    }
    fn value(&mut self) -> Option<String> {
        let result = (|| match self.at() {
            BIG | COMPACT | ANON => {
                let token = self.consume();
                let raw = &self.source[token.start..token.end];
                let n = if token.kind == ANON { 1 } else { 2 };
                let units = Text::from(raw).0;
                let body = units
                    .get(n..units.len().saturating_sub(n))
                    .and_then(|s| Text(s.to_vec()).valid_string());
                let valid=body.as_ref().is_some_and(|body|dictionary::tokens(body,token.kind==COMPACT,true,self.group.delimiters).is_ok_and(|tokens|!tokens.iter().any(|t|matches!(t,super::Token::Tag{expr,..} if expr.starts_with('@') && expr!="@end"))));
                if !valid {
                    self.fatal = true;
                }
                Ok(Some(raw.to_owned()))
            }
            STRING | TRUE | FALSE => {
                let token = self.consume();
                Ok(Some(self.source[token.start..token.end].to_owned()))
            }
            LB => {
                self.take(LB, 0x1000)?;
                self.take(RB, 2)?;
                Ok(Some("[]".into()))
            }
            ID if self.look(0).text == "key" => {
                self.take(ID, 2)?;
                Ok(Some("key".into()))
            }
            _ => Err(()),
        })();
        match result {
            Ok(value) => value,
            Err(()) => {
                self.recover();
                None
            }
        }
    }
}
pub(super) fn scan(source: &str) -> Group {
    let mut parser = Parser {
        source,
        cursor: 0,
        tokens: Vec::new(),
        position: 0,
        follows: Vec::new(),
        last_error: None,
        fatal: false,
        group: Group {
            delimiters: delimiters::Delimiters::default(),
            ranges: Vec::new(),
            headers: Vec::new(),
            dictionaries: Vec::new(),
            aliases: Vec::new(),
        },
    };
    parser.parse();
    parser.group
}
