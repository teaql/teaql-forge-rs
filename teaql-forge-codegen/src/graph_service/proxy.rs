//! The pinned JDK DefaultProxySelector's HTTPS settings and HttpClient filter.
use serde::Serialize;

const DEFAULT_BYPASS: &str = "localhost|127.*|[::1]|0.0.0.0|[::0]";
const DIGITS: &[u8] = include_bytes!("../data/java-character-digit-25.bin");

#[derive(Debug, Serialize)]
pub(super) struct Address {
    pub host: String,
    pub port: u16,
}

impl Address {
    pub fn uri(&self) -> Result<hyper::Uri, ()> {
        // InetSocketAddress treats these as literal host characters, never URL
        // userinfo/path components. Such hosts cannot resolve in the source.
        if self
            .host
            .chars()
            .any(|ch| matches!(ch, '@' | '/' | '?' | '#'))
        {
            return Err(());
        }
        let host = if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("http://{host}:{}", self.port)
            .parse()
            .map_err(|_| ())
    }
}

fn integer(value: &str) -> Option<i32> {
    let (negative, number) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    let (radix, number) = if number.starts_with("0x") || number.starts_with("0X") {
        (16, &number[2..])
    } else if let Some(rest) = number.strip_prefix('#') {
        (16, rest)
    } else if number.starts_with('0') && number.len() > 1 {
        (8, &number[1..])
    } else {
        (10, number)
    };
    if number.is_empty() {
        return None;
    }
    let mut total = 0i64;
    for unit in number.encode_utf16() {
        let digit = match unit {
            65..=70 => unit - 65 + 10,
            97..=102 => unit - 97 + 10,
            0xff21..=0xff26 => unit - 0xff21 + 10,
            0xff41..=0xff46 => unit - 0xff41 + 10,
            _ => u16::from(DIGITS[usize::from(unit)]),
        };
        if u32::from(digit) >= radix {
            return None;
        }
        total = total
            .checked_mul(i64::from(radix))?
            .checked_add(i64::from(digit))?;
    }
    i32::try_from(if negative { -total } else { total }).ok()
}

fn bypass(mask: &str, host: &str) -> bool {
    let wildcard = r"[^\n\r\x{85}\x{2028}\x{2029}]*";
    let host = host.to_lowercase();
    mask.split('|').filter(|part| !part.is_empty()).any(|part| {
        let part = part.to_lowercase();
        let pattern = if part == "*" {
            wildcard.to_string()
        } else if part.starts_with('*') && part.ends_with('*') {
            format!(
                "{wildcard}{}{wildcard}",
                regex::escape(&part[1..part.len() - 1])
            )
        } else if let Some(rest) = part.strip_prefix('*') {
            format!("{wildcard}{}", regex::escape(rest))
        } else if let Some(rest) = part.strip_suffix('*') {
            format!("{}{wildcard}", regex::escape(rest))
        } else {
            regex::escape(&part)
        };
        regex::Regex::new(&format!(r"\A{pattern}\z"))
            .unwrap()
            .is_match(&host)
    })
}

pub(super) fn select(
    host: &str,
    property: impl Fn(&str) -> Option<String>,
) -> Result<Option<Address>, String> {
    let prefixes = ["https.proxy", "proxy", "socksProxy"];
    let selected = prefixes.iter().enumerate().find_map(|(index, prefix)| {
        property(&format!("{prefix}Host"))
            .filter(|host| !host.is_empty())
            .map(|host| (index, host))
    });
    let Some((index, proxy_host)) = selected else {
        return Ok(None);
    };
    let mask = match property("http.nonProxyHosts") {
        Some(mask) if mask.is_empty() => mask,
        Some(mask) => format!("{mask}|{DEFAULT_BYPASS}"),
        None => DEFAULT_BYPASS.to_string(),
    };
    if bypass(&mask, host) {
        return Ok(None);
    }
    let port_at = |index: usize| {
        property(&format!("{}Port", prefixes[index]))
            .and_then(|value| integer(&value))
            .unwrap_or(0)
    };
    let mut port = port_at(index);
    if port == 0 && index < 2 {
        for other in 0..2 {
            if other != index && port == 0 {
                port = port_at(other);
            }
        }
    }
    if port == 0 {
        port = if index == 2 { 1080 } else { 443 };
    }
    let port = u16::try_from(port).map_err(|_| format!("port out of range:{port}"))?;
    // JDK HttpRequestImpl keeps only HTTP proxies. Validate the selector's
    // address before filtering SOCKS, including its unchecked port failures.
    if index == 2 {
        return Ok(None);
    }
    Ok(Some(Address {
        host: proxy_host,
        port,
    }))
}
