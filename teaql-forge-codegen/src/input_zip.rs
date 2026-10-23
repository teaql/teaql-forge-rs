//! ZIP local-stream semantics of the fixed source's java.util.zip.ZipInputStream.
//! Central-directory metadata is deliberately irrelevant to that reader.
use flate2::{Decompress, FlushDecompress, Status};

pub(crate) fn is_zip(bytes: &[u8], filename: &str) -> bool {
    static TYPES: std::sync::OnceLock<Vec<(String, String)>> = std::sync::OnceLock::new();
    let types = TYPES.get_or_init(|| {
        serde_json::from_str(include_str!("data/source-file-types-5.7.11.json")).unwrap()
    });
    let header = bytes
        .iter()
        .take(28)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let extension = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    let detected = types
        .iter()
        .find(|(signature, _)| header.starts_with(&signature.to_ascii_lowercase()))
        .map(|(_, kind)| kind.as_str())
        .unwrap_or(&extension);
    let eq = crate::input_path::eq_ignore_case;
    if eq(detected, "zip") {
        !["docx", "xlsx", "pptx", "war", "ofd"]
            .iter()
            .any(|kind| eq(&extension, kind))
    } else if eq(detected, "jar") {
        !["xlsx", "docx"].iter().any(|kind| eq(&extension, kind))
    } else {
        false
    }
}

fn u16_at(bytes: &[u8], position: usize) -> u16 {
    u16::from_le_bytes(bytes[position..position + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], position: usize) -> u32 {
    u32::from_le_bytes(bytes[position..position + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], position: usize) -> u64 {
    u64::from_le_bytes(bytes[position..position + 8].try_into().unwrap())
}

fn zip_error(message: impl std::fmt::Display) -> String {
    format!("ZipException: {message}")
}

fn take<'a>(bytes: &'a [u8], position: &mut usize, length: usize) -> Result<&'a [u8], String> {
    let end = position.checked_add(length).ok_or("EOFException: null")?;
    let value = bytes.get(*position..end).ok_or("EOFException: null")?;
    *position = end;
    Ok(value)
}

fn inflate(bytes: &[u8]) -> Result<(Vec<u8>, usize), String> {
    let mut inflater = Decompress::new(false);
    let mut output = Vec::new();
    loop {
        let before = (inflater.total_in(), inflater.total_out());
        let mut block = [0; 8192];
        let status = inflater
            .decompress(
                &bytes[before.0 as usize..],
                &mut block,
                FlushDecompress::None,
            )
            .map_err(|error| zip_error(error.message().unwrap_or("invalid compressed data")))?;
        output.extend_from_slice(&block[..(inflater.total_out() - before.1) as usize]);
        if status == Status::StreamEnd {
            return Ok((output, inflater.total_in() as usize));
        }
        if before == (inflater.total_in(), inflater.total_out()) {
            return Err("EOFException: Unexpected end of ZLIB input stream".into());
        }
    }
}

pub(crate) fn read<F>(bytes: &[u8], mut accept: F) -> Result<(), String>
where
    F: FnMut(String, Vec<u8>),
{
    let mut position: usize = 0;
    loop {
        // JDK readLOC treats an incomplete fixed header as end of stream, even
        // when it begins with the local signature. Names and extras require EOF.
        let Some(header) = bytes.get(position..position.saturating_add(30)) else {
            return Ok(());
        };
        if u32_at(header, 0) != 0x04034b50 {
            return Ok(());
        }
        position += 30;
        let flags = u16_at(header, 6);
        let method = u16_at(header, 8);
        let mut crc = u32_at(header, 14);
        let mut compressed_size = u32_at(header, 18) as u64;
        let mut size = u32_at(header, 22) as u64;
        let name = take(bytes, &mut position, u16_at(header, 26) as usize)?;
        // ZipInputStream's default charset is UTF-8 regardless of EFS bit 11.
        let name = std::str::from_utf8(name)
            .map_err(|_| zip_error("invalid LOC header (bad entry name)"))?
            .to_string();
        if flags & 1 != 0 {
            return Err(zip_error("encrypted ZIP entry not supported"));
        }
        if flags & 8 != 0 && method != 8 {
            return Err(zip_error("only DEFLATED entries can have EXT descriptor"));
        }
        let extra = take(bytes, &mut position, u16_at(header, 28) as usize)?;
        if flags & 8 == 0 {
            let mut cursor = 0;
            while cursor + 4 <= extra.len() {
                let tag = u16_at(extra, cursor);
                let length = u16_at(extra, cursor + 2) as usize;
                cursor += 4;
                let Some(data) = extra.get(cursor..cursor + length) else {
                    break;
                };
                if tag == 1 {
                    let mut offset = 0;
                    if size == u32::MAX as u64 && data.len() >= 8 {
                        size = u64_at(data, 0);
                        offset = 8;
                    }
                    if compressed_size == u32::MAX as u64 && data.len() >= offset + 8 {
                        compressed_size = u64_at(data, offset);
                    }
                }
                cursor += length;
            }
        }
        let (content, consumed) = match method {
            0 => {
                let length = usize::try_from(size).map_err(|_| zip_error("unexpected EOF"))?;
                let content = take(bytes, &mut position, length)
                    .map_err(|_| zip_error("unexpected EOF"))?
                    .to_vec();
                (content, length)
            }
            8 => {
                let (content, length) = inflate(&bytes[position..])?;
                position += length;
                (content, length)
            }
            _ => return Err(zip_error("invalid compression method")),
        };
        if flags & 8 != 0 {
            // JDK selects ZIP64 descriptors from actual inflater byte counts;
            // a small ZIP64 stream's 64-bit descriptor is read as a 32-bit one.
            let zip64 = content.len() as u64 > u32::MAX as u64 || consumed as u64 > u32::MAX as u64;
            let descriptor = take(bytes, &mut position, if zip64 { 24 } else { 16 })?;
            let offset = if u32_at(descriptor, 0) == 0x08074b50 {
                4
            } else {
                position -= 4;
                0
            };
            crc = u32_at(descriptor, offset);
            compressed_size = if zip64 {
                u64_at(descriptor, offset + 4)
            } else {
                u32_at(descriptor, offset + 4) as u64
            };
            size = if zip64 {
                u64_at(descriptor, offset + 12)
            } else {
                u32_at(descriptor, offset + 8) as u64
            };
        }
        if method == 8 && size != content.len() as u64 {
            return Err(zip_error(format!(
                "invalid entry size (expected {size} but got {} bytes)",
                content.len()
            )));
        }
        if method == 8 && compressed_size != consumed as u64 {
            return Err(zip_error(format!("invalid entry compressed size (expected {compressed_size} but got {consumed} bytes)")));
        }
        let actual_crc = crc32fast::hash(&content);
        if crc != actual_crc && !(method == 0 && content.is_empty()) {
            return Err(zip_error(format!(
                "invalid entry CRC (expected 0x{crc:x} but got 0x{actual_crc:x})"
            )));
        }
        // All entries, including ignored hidden/directory entries, must first
        // be consumed and validated when advancing the stream.
        accept(name, content);
    }
}
