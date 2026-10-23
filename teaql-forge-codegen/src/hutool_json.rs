//! JSONTokener/JSONWriter behavior used by the pinned Hutool 5.7.11 generator.
//! Numbers stay decimal strings: parsing through f64 loses source precision.
use std::fmt::Write;

#[derive(Clone)]
enum Value {
    Null,
    Bool(bool),
    Number(Result<String, String>, String), // compact writer form, Java Number.toString()
    String(Vec<u16>),
    Array(Vec<Value>),
    Object(Vec<(Vec<u16>, Value)>),
}

fn quote(value: &[u16]) -> String {
    let mut out = String::from("\"");
    let mut plain = Vec::new();
    for &ch in value {
        let escaped = match ch {
            8 => Some("\\b"),
            9 => Some("\\t"),
            10 => Some("\\n"),
            12 => Some("\\f"),
            13 => Some("\\r"),
            34 => Some("\\\""),
            92 => Some("\\\\"),
            _ => None,
        };
        if escaped.is_some()
            || ch < 32
            || matches!(ch, 0x80..=0xa0 | 0x2000..=0x2010 | 0x2028..=0x202f | 0x2066..=0x206f)
        {
            out.extend(char::decode_utf16(plain.drain(..)).map(|r| r.unwrap_or('?')));
            if let Some(escape) = escaped {
                out.push_str(escape);
            } else {
                write!(out, "\\u{ch:04x}").unwrap();
            }
        } else {
            plain.push(ch);
        }
    }
    out.extend(char::decode_utf16(plain).map(|r| r.unwrap_or('?')));
    out.push('"');
    out
}

impl Value {
    fn compact(&self) -> Result<String, String> {
        Ok(match self {
            Self::Null => "null".into(),
            Self::Bool(v) => v.to_string(),
            Self::Number(v, _) => return v.clone(),
            Self::String(v) => quote(v),
            Self::Array(v) => format!(
                "[{}]",
                v.iter()
                    .filter(|v| !matches!(v, Self::Null))
                    .map(Self::compact)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(",")
            ),
            Self::Object(v) => format!(
                "{{{}}}",
                hash_order(v)
                    .into_iter()
                    .map(|i| Ok(format!("{}:{}", quote(&v[i].0), v[i].1.compact()?)))
                    .collect::<Result<Vec<_>, String>>()?
                    .join(",")
            ),
        })
    }
    fn key(&self) -> Result<Vec<u16>, String> {
        Ok(match self {
            Self::String(v) => v.clone(),
            Self::Number(_, v) => v.encode_utf16().collect(),
            _ => self.compact()?.encode_utf16().collect(),
        })
    }
}

// Java HashMap encounter order, including red/black collision bins. JSONObject
// defaults to an unordered HashMap; sorting keys or preserving input order differs.
#[derive(Clone, Default)]
struct TreeNode {
    parent: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    red: bool,
}
fn rotate(nodes: &mut [TreeNode], root: &mut usize, p: usize, left: bool) {
    let r = if left {
        nodes[p].right.unwrap()
    } else {
        nodes[p].left.unwrap()
    };
    let child = if left { nodes[r].left } else { nodes[r].right };
    if left {
        nodes[p].right = child;
    } else {
        nodes[p].left = child;
    }
    if let Some(c) = child {
        nodes[c].parent = Some(p);
    }
    let pp = nodes[p].parent;
    nodes[r].parent = pp;
    if let Some(pp) = pp {
        if nodes[pp].left == Some(p) {
            nodes[pp].left = Some(r);
        } else {
            nodes[pp].right = Some(r);
        }
    } else {
        *root = r;
    }
    if left {
        nodes[r].left = Some(p);
    } else {
        nodes[r].right = Some(p);
    }
    nodes[p].parent = Some(r);
}
fn balance(nodes: &mut [TreeNode], root: &mut usize, mut x: usize) {
    nodes[x].red = true;
    loop {
        let Some(p) = nodes[x].parent else {
            nodes[x].red = false;
            *root = x;
            return;
        };
        if !nodes[p].red {
            return;
        }
        let Some(g) = nodes[p].parent else {
            return;
        };
        let left = nodes[g].left == Some(p);
        let uncle = if left { nodes[g].right } else { nodes[g].left };
        if uncle.is_some_and(|u| nodes[u].red) {
            nodes[uncle.unwrap()].red = false;
            nodes[p].red = false;
            nodes[g].red = true;
            x = g;
        } else {
            if (if left { nodes[p].right } else { nodes[p].left }) == Some(x) {
                rotate(nodes, root, p, left);
                x = p;
            }
            let p = nodes[x].parent.unwrap();
            let g = nodes[p].parent.unwrap();
            nodes[p].red = false;
            nodes[g].red = true;
            rotate(nodes, root, g, !left);
            return;
        }
    }
}
fn tree_insert(
    nodes: &mut [TreeNode],
    root: &mut usize,
    x: usize,
    keys: &[(Vec<u16>, Value)],
    hashes: &[i32],
) -> usize {
    let mut p = *root;
    loop {
        let left = hashes[x]
            .cmp(&hashes[p])
            .then_with(|| keys[x].0.cmp(&keys[p].0))
            .is_lt();
        let child = if left { nodes[p].left } else { nodes[p].right };
        if let Some(c) = child {
            p = c;
        } else {
            nodes[x] = TreeNode {
                parent: Some(p),
                ..Default::default()
            };
            if left {
                nodes[p].left = Some(x);
            } else {
                nodes[p].right = Some(x);
            }
            balance(nodes, root, x);
            return p;
        }
    }
}
fn treeify(
    bin: &mut Vec<usize>,
    nodes: &mut [TreeNode],
    keys: &[(Vec<u16>, Value)],
    hashes: &[i32],
) {
    let mut root = bin[0];
    nodes[root] = TreeNode::default();
    for &x in &bin[1..] {
        tree_insert(nodes, &mut root, x, keys, hashes);
    }
    let position = bin.iter().position(|&v| v == root).unwrap();
    bin.remove(position);
    bin.insert(0, root);
}
fn hash_order(values: &[(Vec<u16>, Value)]) -> Vec<usize> {
    let hashes = values
        .iter()
        .map(|(key, _)| {
            let h = key
                .iter()
                .fold(0_u32, |h, &c| h.wrapping_mul(31).wrapping_add(u32::from(c)));
            (h ^ (h >> 16)) as i32
        })
        .collect::<Vec<_>>();
    let mut nodes = vec![TreeNode::default(); values.len()];
    let mut bins = vec![Vec::new(); 32];
    let mut trees = vec![false; 32];
    for (i, &h) in hashes.iter().enumerate() {
        let b = h as usize & (bins.len() - 1);
        if trees[b] {
            let mut root = bins[b][0];
            let parent = tree_insert(&mut nodes, &mut root, i, values, &hashes);
            let position = bins[b].iter().position(|&v| v == parent).unwrap();
            bins[b].insert(position + 1, i);
            let position = bins[b].iter().position(|&v| v == root).unwrap();
            bins[b].remove(position);
            bins[b].insert(0, root);
        } else {
            bins[b].push(i);
            if bins[b].len() >= 9 && bins.len() >= 64 {
                treeify(&mut bins[b], &mut nodes, values, &hashes);
                trees[b] = true;
            }
        }
        if i + 1 > bins.len() * 3 / 4 || (!trees[b] && bins[b].len() >= 9 && bins.len() < 64) {
            let old_len = bins.len();
            let mut new_bins = vec![Vec::new(); old_len * 2];
            let mut new_trees = vec![false; old_len * 2];
            for (b, bin) in bins.iter().enumerate() {
                for &x in bin {
                    new_bins[hashes[x] as usize & (old_len * 2 - 1)].push(x);
                }
                if trees[b] {
                    for j in [b, b + old_len] {
                        if new_bins[j].len() > 6 {
                            new_trees[j] = true;
                            if !new_bins[if j == b { b + old_len } else { b }].is_empty() {
                                treeify(&mut new_bins[j], &mut nodes, values, &hashes);
                            }
                        }
                    }
                }
            }
            bins = new_bins;
            trees = new_trees;
        }
    }
    bins.into_iter().flatten().collect()
}

const DIGITS: &[u8] = include_bytes!("data/java-character-digit-25.bin");
pub(crate) fn parse_integer(value: &str) -> Option<i64> {
    let units = value.encode_utf16().collect::<Vec<_>>();
    let mut slice = units.as_slice();
    let negative = slice.first() == Some(&45);
    if negative || slice.first() == Some(&43) {
        slice = &slice[1..];
    }
    if slice.is_empty() {
        return None;
    }
    let mut result = 0_i64;
    for &ch in slice {
        let digit = DIGITS[usize::from(ch)];
        if digit > 9 {
            return None;
        }
        result = result.checked_mul(10)?.checked_sub(i64::from(digit))?;
    }
    if negative {
        Some(result)
    } else {
        result.checked_neg()
    }
}

fn decimal(raw: &str) -> Option<(Result<String, String>, String)> {
    let (mantissa, exponent) = raw
        .find(['e', 'E'])
        .map_or((raw, "0"), |p| (&raw[..p], &raw[p + 1..]));
    let exponent = parse_integer(exponent)?;
    if !(-2147483647..=2147483648).contains(&exponent) {
        return None;
    }
    let (negative, mantissa) = if let Some(tail) = mantissa.strip_prefix('-') {
        (true, tail)
    } else {
        (false, mantissa.strip_prefix('+').unwrap_or(mantissa))
    };
    let mut digits = String::new();
    let mut fraction = None;
    for ch in mantissa.encode_utf16() {
        if ch == 46 {
            if fraction.is_some() {
                return None;
            }
            fraction = Some(0_i32);
        } else {
            let d = DIGITS[usize::from(ch)];
            if d > 9 {
                return None;
            }
            digits.push(char::from(b'0' + d));
            if let Some(n) = &mut fraction {
                *n += 1;
            }
        }
    }
    if digits.is_empty() {
        return None;
    }
    let scale = i32::try_from(i64::from(fraction.unwrap_or(0)) - exponent).ok()?;
    let digits = digits.trim_start_matches('0');
    let zero = digits.is_empty();
    let digits = if zero { "0" } else { digits };
    let sign = if negative && !zero { "-" } else { "" };
    // BigDecimal.toString() switches to exponent notation at adjusted exponent < -6.
    let adjusted = digits.len() as i64 - 1 - i64::from(scale);
    let java = if scale >= 0 && adjusted >= -6 {
        plain(digits, scale, sign).ok()?
    } else {
        let exponent = if adjusted >= 0 {
            format!("+{adjusted}")
        } else {
            adjusted.to_string()
        };
        format!(
            "{sign}{}{}E{exponent}",
            &digits[..1],
            if digits.len() == 1 {
                String::new()
            } else {
                format!(".{}", &digits[1..])
            }
        )
    };
    if zero {
        return Some((Ok("0".into()), java));
    }
    let trimmed = digits.trim_end_matches('0');
    let compact = scale
        .checked_sub((digits.len() - trimmed.len()) as i32)
        .ok_or_else(|| "BigDecimal scale overflow".to_string())
        .and_then(|scale| plain(trimmed, scale, sign));
    Some((compact, java))
}
fn plain(digits: &str, scale: i32, sign: &str) -> Result<String, String> {
    let point = digits.len() as i64 - i64::from(scale);
    // BigDecimal.toPlainString uses Math.negateExact(scale), then a Java String.
    let length = if point <= 0 {
        sign.len() as i64 + 2 - point + digits.len() as i64
    } else {
        sign.len() as i64 + point.max(digits.len() as i64 + i64::from(scale > 0))
    };
    if scale == i32::MIN || length > i64::from(i32::MAX) {
        return Err("BigDecimal cannot fit in a Java String".into());
    }
    Ok(if point <= 0 {
        format!("{sign}0.{}{digits}", "0".repeat((-point) as usize))
    } else if point >= digits.len() as i64 {
        format!(
            "{sign}{digits}{}",
            "0".repeat(point as usize - digits.len())
        )
    } else {
        format!(
            "{sign}{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    })
}
fn atom(raw: Vec<u16>) -> Result<Value, String> {
    let text = String::from_utf16_lossy(&raw);
    Ok(if crate::input_path::eq_ignore_case(&text, "null") {
        Value::Null
    } else if crate::input_path::eq_ignore_case(&text, "true") {
        Value::Bool(true)
    } else if crate::input_path::eq_ignore_case(&text, "false") {
        Value::Bool(false)
    } else {
        if text.starts_with(|ch: char| ch.is_ascii_digit() || ch == '-') {
            if text.contains(['.', 'e', 'E']) {
                if let Some((compact, java)) = decimal(&text) {
                    return Ok(Value::Number(compact, java));
                }
            } else if let Some(n) = parse_integer(&text) {
                if text == n.to_string() {
                    return Ok(Value::Number(Ok(text.clone()), text));
                }
            }
        }
        Value::String(raw)
    })
}
struct Parser {
    input: Vec<u16>,
    position: usize,
}
impl Parser {
    fn next(&mut self) -> u16 {
        let ch = self.input.get(self.position).copied().unwrap_or(0);
        if self.position < self.input.len() {
            self.position += 1;
        }
        ch
    }
    fn clean(&mut self) -> u16 {
        loop {
            let ch = self.next();
            if ch == 0 || ch > 32 {
                return ch;
            }
        }
    }
    fn back(&mut self) {
        self.position = self.position.saturating_sub(1);
    }
    fn value(&mut self) -> Result<Value, String> {
        let ch = self.clean();
        match ch {
            34 | 39 => {
                let mut out = Vec::new();
                loop {
                    let c = self.next();
                    match c {
                        0 | 10 | 13 => return Err("Unterminated string".into()),
                        92 => {
                            let c = self.next();
                            out.push(match c {
                                98 => 8,
                                116 => 9,
                                110 => 10,
                                102 => 12,
                                114 => 13,
                                34 | 39 | 92 | 47 => c,
                                117 => {
                                    let mut units = [0; 4];
                                    for unit in &mut units {
                                        *unit = self.next();
                                    }
                                    let negative = units[0] == 45;
                                    let digits = if negative || units[0] == 43 {
                                        &units[1..]
                                    } else {
                                        &units[..]
                                    };
                                    let mut unit = 0_i32;
                                    for &c in digits {
                                        let d = DIGITS[usize::from(c)];
                                        if d > 15 {
                                            return Err("Invalid unicode escape".into());
                                        }
                                        unit = unit * 16 + i32::from(d);
                                    }
                                    (if negative { -unit } else { unit }) as u16
                                }
                                _ => return Err("Illegal escape".into()),
                            });
                        }
                        c if c == ch => return Ok(Value::String(out)),
                        c => out.push(c),
                    }
                }
            }
            91 => {
                let mut out = Vec::new();
                if self.clean() == 93 {
                    return Ok(Value::Array(out));
                }
                self.back();
                loop {
                    if self.clean() == 44 {
                        self.back();
                        out.push(Value::Null);
                    } else {
                        self.back();
                        out.push(self.value()?);
                    }
                    match self.clean() {
                        93 => return Ok(Value::Array(out)),
                        44 => {
                            if self.clean() == 93 {
                                return Ok(Value::Array(out));
                            }
                            self.back();
                        }
                        _ => return Err("Expected a ',' or ']'".into()),
                    }
                }
            }
            123 => {
                let mut out: Vec<(Vec<u16>, Value)> = Vec::new();
                loop {
                    match self.clean() {
                        125 => return Ok(Value::Object(out)),
                        0 => return Err("A JSONObject text must end with '}'".into()),
                        _ => self.back(),
                    }
                    let key = self.value()?.key()?;
                    if self.clean() != 58 {
                        return Err("Expected a ':' after a key".into());
                    }
                    let value = self.value()?;
                    if out.iter().any(|(k, _)| *k == key) {
                        return Err("Duplicate key".into());
                    }
                    if !matches!(value, Value::Null) {
                        out.push((key, value));
                    }
                    match self.clean() {
                        125 => return Ok(Value::Object(out)),
                        44 | 59 => {
                            if self.clean() == 125 {
                                return Ok(Value::Object(out));
                            }
                            self.back();
                        }
                        _ => return Err("Expected a ',' or '}'".into()),
                    }
                }
            }
            _ => {
                let mut raw = Vec::new();
                let mut c = ch;
                while c >= 32 && !(c <= 127 && b",:]}/\\\"[{;=#".contains(&(c as u8))) {
                    raw.push(c);
                    c = self.next();
                }
                if c != 0 {
                    self.back();
                }
                while raw.last().is_some_and(|&c| c <= 32) {
                    raw.pop();
                }
                if raw.is_empty() {
                    return Err("Missing value".into());
                }
                atom(raw)
            }
        }
    }
}
pub(crate) fn compact(value: &str) -> Result<String, String> {
    let mut parser = Parser {
        input: value
            .trim_matches(crate::input_path::blank_char)
            .encode_utf16()
            .collect(),
        position: 0,
    };
    parser.value().and_then(|v| v.compact())
}
