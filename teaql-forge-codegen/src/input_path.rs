//! Hutool 5.7.11 FileUtil.normalize, before GenerateContext strips boundary '/'.

pub(crate) fn normalize(path: &str) -> String {
    let home = if path.contains('~') {
        crate::process_properties::user_home()
    } else {
        String::new()
    };
    normalize_for_home(path, &home)
}

fn normalize_for_home(original: &str, home: &str) -> String {
    let mut path = original;
    for prefix in ["classpath:", "file:"] {
        // Hutool removes these prefixes using lower-case strings, whereas
        // file-type checks use String.equalsIgnoreCase. Their Unicode rules
        // differ (e.g. dotless i must remain in a path prefix).
        if path
            .get(..prefix.len())
            .is_some_and(|value| value.eq_ignore_ascii_case(prefix))
        {
            path = &path[prefix.len()..];
        }
    }
    let expanded;
    if let Some(suffix) = path.strip_prefix('~') {
        expanded = format!("{home}{suffix}");
        path = &expanded;
    }
    let mut collapsed = String::new();
    let mut slash = false;
    for ch in path.chars() {
        if ch == '/' || ch == '\\' {
            if !slash {
                collapsed.push('/');
            }
            slash = true;
        } else {
            collapsed.push(ch);
            slash = false;
        }
    }
    let mut path = collapsed.trim_start_matches(blank_char).to_string();
    if original.starts_with("\\\\") {
        path.insert(0, '\\');
    }
    let mut prefix = String::new();
    if let Some(colon) = path.find(':') {
        let candidate = path[..=colon].strip_prefix('/').unwrap_or(&path[..=colon]);
        if !candidate.contains('/') {
            prefix = candidate.into();
            path = path[colon + 1..].to_string();
        }
    }
    if let Some(rest) = path.strip_prefix('/') {
        prefix.push('/');
        path = rest.into();
    }
    let mut elements = Vec::new();
    let mut parents = 0;
    for element in path.split('/').rev() {
        match element {
            "." => (),
            ".." => parents += 1,
            _ if parents > 0 => parents -= 1,
            _ => elements.push(element),
        }
    }
    if prefix.is_empty() {
        elements.extend(std::iter::repeat_n("..", parents));
    }
    elements.reverse();
    prefix + &elements.join("/")
}

// Java's case-insensitive comparisons against ASCII tokens also accept these
// Unicode case equivalents. ASCII byte folding alone misses them.
fn same_char(ch: char, expected: char) -> bool {
    ch.eq_ignore_ascii_case(&expected)
        || matches!(
            (ch, expected.to_ascii_lowercase()),
            ('\u{0130}' | '\u{0131}', 'i') | ('\u{017f}', 's') | ('\u{212a}', 'k')
        )
}

fn strip_prefix_ignore_case<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    let mut chars = input.char_indices();
    let mut end = 0;
    for expected in prefix.chars() {
        let (offset, ch) = chars.next()?;
        if !same_char(ch, expected) {
            return None;
        }
        end = offset + ch.len_utf8();
    }
    Some(&input[end..])
}

pub(crate) fn eq_ignore_case(input: &str, ascii_token: &str) -> bool {
    strip_prefix_ignore_case(input, ascii_token).is_some_and(str::is_empty)
}

pub(crate) fn blank_char(ch: char) -> bool {
    matches!(ch, '\0' | '\u{0009}'..='\u{000d}' | '\u{001c}'..='\u{0020}'
        | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'..='\u{202a}'
        | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

#[cfg(unix)]
pub(crate) fn os_user_home() -> String {
    use std::ffi::CStr;
    // OpenJDK obtains its default user.home from the OS user record. HOME can
    // be changed independently, so reading that variable changes ~ semantics.
    let mut buffer = vec![0_u8; 16_384];
    loop {
        let mut record = std::mem::MaybeUninit::<libc::passwd>::uninit();
        let mut result = std::ptr::null_mut();
        // Buffer, record and result remain live throughout getpwuid_r. The
        // returned pw_dir points into buffer and is copied before it is freed.
        let status = unsafe {
            libc::getpwuid_r(
                libc::getuid(),
                record.as_mut_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut result,
            )
        };
        if status == libc::ERANGE {
            buffer.resize(buffer.len() * 2, 0);
            continue;
        }
        if status == 0 && !result.is_null() {
            return unsafe { CStr::from_ptr((*result).pw_dir) }
                .to_string_lossy()
                .into_owned();
        }
        return "?".into();
    }
}

#[cfg(not(unix))]
pub(crate) fn os_user_home() -> String {
    std::env::var("USERPROFILE").unwrap_or_else(|_| "?".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hutool_paths_and_all_bmp_blank_characters_match_source() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/expected/path-normalization-6f2d0a.json"
        ))
        .unwrap();
        let home = reference["user_home"].as_str().unwrap();
        for case in reference["paths"].as_array().unwrap() {
            let path = case["input"].as_str().unwrap();
            assert_eq!(
                normalize_for_home(path, home),
                case["normalized"],
                "{path:?}"
            );
        }
        let actual = (0..=0xffff)
            .filter_map(char::from_u32)
            .filter(|ch| blank_char(*ch))
            .map(|ch| ch as u32)
            .collect::<Vec<_>>();
        assert_eq!(serde_json::json!(actual), reference["blank_chars"]);
        for (letter, expected) in reference["ascii_case_characters"].as_object().unwrap() {
            let expected_char = letter.chars().next().unwrap();
            let actual = (0..=0xffff)
                .filter_map(char::from_u32)
                .filter(|ch| same_char(*ch, expected_char))
                .map(|ch| ch as u32)
                .collect::<Vec<_>>();
            assert_eq!(
                serde_json::json!(actual),
                *expected,
                "ASCII case equivalents for {letter}"
            );
        }
    }
}
