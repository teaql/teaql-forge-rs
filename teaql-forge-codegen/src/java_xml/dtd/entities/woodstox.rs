//! Woodstox locations belong to replacement streams, with their caller chain.
use crate::java_xml::Failure;
use std::sync::Arc;

#[derive(Clone)]
struct Location {
    row: usize,
    column: usize,
    system_id: Option<String>,
}

impl Location {
    fn render(&self) -> String {
        if let Some(id) = &self.system_id {
            format!("[row,col,system-id]: [{},{},\"{id}\"]", self.row, self.column)
        } else {
            format!("[row,col {{unknown-source}}]: [{},{}]", self.row, self.column)
        }
    }
}

#[derive(Clone, Copy)]
struct Mark {
    offset: usize,
    row: usize,
    column: usize,
    cr: bool,
}

struct IndexedText {
    value: String,
    marks: Vec<Mark>,
}

impl IndexedText {
    fn new(value: &str) -> Arc<Self> {
        let mut mark = Mark { offset: 0, row: 1, column: 0, cr: false };
        let mut marks = vec![mark];
        for (index, (pos, ch)) in value.char_indices().enumerate() {
            if ch == '\r' || ch == '\n' && !mark.cr { mark.row += 1; mark.column = 0; }
            else if ch != '\n' || !mark.cr { mark.column += ch.len_utf16(); }
            mark.cr = ch == '\r'; mark.offset = pos + ch.len_utf8();
            if (index + 1) % 128 == 0 { marks.push(mark); }
        }
        Arc::new(Self { value: value.into(), marks })
    }
}

#[derive(Clone)]
pub(super) struct Origin {
    text: Arc<IndexedText>,
    offset: usize,
    start: Location,
    callers: Vec<Location>,
}

impl Origin {
    pub fn line(&self) -> usize { self.location().row }
    pub fn input_start() -> Self {
        Self { text: IndexedText::new(""), offset: 0, start: Location { row: 1, column: 1, system_id: None }, callers: Vec::new() }
    }

    pub fn document(text: &str, offset: usize) -> Self {
        Self { text: IndexedText::new(text), offset, start: Location { row: 1, column: 0, system_id: None }, callers: Vec::new() }
    }

    pub fn external(text: &str, offset: usize, url: &reqwest::Url) -> Self {
        let id = url.as_str().strip_prefix("file:///").map(|p| format!("file:/{p}")).unwrap_or_else(|| url.to_string());
        let mut result = Self::document(text, offset);
        result.start.system_id = Some(id);
        result
    }

    pub fn shifted(&self, offset: usize) -> Self {
        let mut result = self.clone();
        result.offset += offset;
        result
    }

    fn location(&self) -> Location {
        let text = &self.text.value;
        let mut end = self.offset.min(text.len());
        while !text.is_char_boundary(end) { end -= 1; }
        let mark = self.text.marks[self.text.marks.partition_point(|m| m.offset <= end).saturating_sub(1)];
        let mut result = self.start.clone();
        result.row += mark.row - 1;
        result.column = if mark.row == 1 { result.column + mark.column } else { mark.column };
        let mut cr = mark.cr;
        for ch in text[mark.offset..end].chars() {
            if ch == '\r' || ch == '\n' && !cr { result.row += 1; result.column = 0; }
            else if ch != '\n' || !cr { result.column += ch.len_utf16(); }
            cr = ch == '\r';
        }
        result.column += self.offset.saturating_sub(text.len());
        result
    }

    pub fn replacement(&self, value: &str) -> Self {
        Self { text: IndexedText::new(value), offset: 0, start: self.location(), callers: self.callers.clone() }
    }

    pub fn called_from(mut self, caller: &Self, lookahead: bool) -> Self {
        let mut caller = caller.clone();
        if lookahead {
            caller.offset += caller.text.value.get(caller.offset..).and_then(|s| s.chars().next()).map(char::len_utf8).unwrap_or(1);
        }
        self.callers = vec![caller.location()];
        self.callers.extend(caller.callers);
        self
    }

    pub fn error(&self, message: impl Into<String>) -> Failure {
        let location = self.location();
        let mut message = format!("SAXParseException: {}\n at {}", message.into(), location.render());
        for caller in &self.callers { message.push_str(&format!("\n from {}", caller.render())); }
        Failure { message, line: Some(location.row), offset: None, factory: None }
    }

    pub fn io_error(&self, error: Failure) -> Failure {
        if let Some(label) = error.message.strip_prefix("UnsupportedEncodingException: ") {
            return super::failure(format!("Unsupported encoding: {label}"));
        }
        if let Some((class, detail)) = error.message.split_once(": ") {
            // Transport failures become WstxIOException with its native detail,
            // unlike a missing local resource's located parsing exception.
            if matches!(class, "ConnectException" | "SocketException" | "UnknownHostException") {
                return super::failure(detail.into());
            }
            if class.ends_with("Exception") {
                let package = if matches!(class, "MalformedURLException" | "ConnectException" | "SocketException" | "UnknownHostException") { "java.net" } else { "java.io" };
                return self.error(format!("(was {package}.{class}) {detail}"));
            }
        }
        error
    }
}

/// SAX start tokens, excluding declarations, comments and character data.
pub(super) fn node_tokens(text: &str) -> (Vec<(usize, String)>, bool) {
    let mut nodes = Vec::new(); let mut pos = 0; let mut ends_markup = false;
    while pos < text.len() {
        let rest = &text[pos..];
        let close = if rest.starts_with("<!--") { Some(("-->", true)) }
            else if rest.starts_with("<?") { Some(("?>", true)) }
            else if rest.starts_with("<![CDATA[") { Some(("]]>", true)) } else { None };
        if let Some((close, markup)) = close {
            pos += rest.find(close).map(|n| n + close.len()).unwrap_or(rest.len());
            ends_markup = markup; continue;
        }
        if rest.starts_with('<') {
            let start = pos; pos += 1;
            let closing = text[pos..].starts_with('/'); if closing { pos += 1; }
            let name_start = pos;
            while pos < text.len() && text[pos..].chars().next().is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '_' | ':' | '-' | '.') || ch as u32 >= 0x300) {
                pos += text[pos..].chars().next().unwrap().len_utf8();
            }
            if !closing && pos > name_start { nodes.push((start, text[name_start..pos].into())); }
            let mut quote = None;
            while pos < text.len() {
                let ch = text[pos..].chars().next().unwrap(); pos += ch.len_utf8();
                if let Some(q) = quote { if ch == q { quote = None; } }
                else if matches!(ch, '\'' | '"') { quote = Some(ch); }
                else if ch == '>' { break; }
            }
            ends_markup = true;
        } else {
            pos += rest.chars().next().unwrap().len_utf8(); ends_markup = false;
        }
    }
    (nodes, ends_markup)
}
