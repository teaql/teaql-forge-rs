//! Group delimiters also govern nested templates and anonymous defaults.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Delimiters {
    pub start: char,
    pub stop: char,
}

impl Default for Delimiters {
    fn default() -> Self {
        Self {
            start: '<',
            stop: '>',
        }
    }
}

impl Delimiters {
    pub fn from_tokens(start: &str, stop: &str) -> Option<Self> {
        let start = start.chars().nth(1)?;
        let stop = stop.chars().nth(1)?;
        let reserved =
            |ch: char| ch.is_ascii_alphanumeric() || matches!(ch, '@' | '-' | '_' | '[' | ']');
        (!reserved(start) && !reserved(stop)).then_some(Self { start, stop })
    }
    pub fn immediate(self, body: &str) -> bool {
        let units = body.encode_utf16().collect::<Vec<_>>();
        units.starts_with(&[self.start_unit(), 40]) && units.ends_with(&[41, self.stop_unit()])
    }

    pub fn start_unit(self) -> u16 {
        self.start.encode_utf16(&mut [0; 2])[0]
    }
    pub fn stop_unit(self) -> u16 {
        self.stop.encode_utf16(&mut [0; 2])[0]
    }

    pub fn stop_is_syntax(self) -> bool {
        // STLexer.inside checks these tokens before its RDELIM branch.
        matches!(
            self.stop,
            ' ' | '\t'
                | '\r'
                | '\n'
                | '.'
                | ','
                | ':'
                | ';'
                | '('
                | ')'
                | '['
                | ']'
                | '='
                | '!'
                | '/'
                | '@'
                | '"'
                | '&'
                | '|'
                | '{'
        )
    }
}
