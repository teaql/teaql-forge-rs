//! Default JSSE trust-store loading for the pinned JDK, without a JVM process.
use foreign_types::ForeignType;
use openssl::{
    hash::{Hasher, MessageDigest},
    pkcs12::Pkcs12,
    x509::X509,
};
use rustls::{ClientConfig, RootCertStore, pki_types::CertificateDer};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::OnceLock,
};
mod keys;
mod serialized;

pub(super) const INITIALIZATION_MESSAGE: &str = "java.io.IOException: java.security.NoSuchAlgorithmException: Error constructing implementation (algorithm: Default, provider: SunJSSE, class: sun.security.ssl.SSLContextImpl$DefaultSSLContext)";

struct Reader<'a>(&'a [u8], usize);
impl<'a> Reader<'a> {
    fn bytes(&mut self, size: usize) -> Result<&'a [u8], ()> {
        let end = self.1.checked_add(size).ok_or(())?;
        let value = self.0.get(self.1..end).ok_or(())?;
        self.1 = end;
        Ok(value)
    }
    fn int(&mut self) -> Result<usize, ()> {
        let value = i32::from_be_bytes(self.bytes(4)?.try_into().unwrap());
        usize::try_from(value).map_err(|_| ())
    }
    fn utf(&mut self) -> Result<Vec<u16>, ()> {
        let length = u16::from_be_bytes(self.bytes(2)?.try_into().unwrap());
        let bytes = self.bytes(usize::from(length))?;
        let (mut index, mut result) = (0, Vec::new());
        while index < bytes.len() {
            let first = bytes[index];
            index += 1;
            let value = if first < 128 {
                u16::from(first)
            } else if first & 0xe0 == 0xc0 {
                let next = *bytes.get(index).ok_or(())?;
                if next & 0xc0 != 0x80 {
                    return Err(());
                }
                index += 1;
                (u16::from(first & 31) << 6) | u16::from(next & 63)
            } else if first & 0xf0 == 0xe0 {
                let next = bytes.get(index..index + 2).ok_or(())?;
                if next.iter().any(|b| b & 0xc0 != 0x80) {
                    return Err(());
                }
                index += 2;
                (u16::from(first & 15) << 12)
                    | (u16::from(next[0] & 63) << 6)
                    | u16::from(next[1] & 63)
            } else {
                return Err(());
            };
            result.push(value);
        }
        Ok(result)
    }
    fn certificate(&mut self, version: usize) -> Result<Vec<u8>, ()> {
        if version == 2 {
            let kind = String::from_utf16(&self.utf()?).map_err(|_| ())?;
            if !kind.eq_ignore_ascii_case("X.509") && !kind.eq_ignore_ascii_case("X509") {
                return Err(());
            }
        }
        let size = self.int()?;
        X509::from_der(self.bytes(size)?)
            .and_then(|cert| cert.to_der())
            .map_err(|_| ())
    }
}

fn jks(bytes: &[u8], password: &str, jceks: bool) -> Result<Vec<Vec<u8>>, ()> {
    let mut reader = Reader(bytes, 0);
    let magic = if jceks {
        [0xce, 0xce, 0xce, 0xce]
    } else {
        [0xfe, 0xed, 0xfe, 0xed]
    };
    let actual = reader.bytes(4)?;
    if actual != magic && !(jceks && actual == [0xfe, 0xed, 0xfe, 0xed]) {
        return Err(());
    }
    let version = reader.int()?;
    if !matches!(version, 1 | 2) {
        return Err(());
    }
    let count = reader.int()?;
    let mut entries = BTreeMap::new();
    for _ in 0..count {
        let tag = reader.int()?;
        let alias = reader.utf()?;
        reader.bytes(8)?;
        let cert = match tag {
            1 => {
                let size = reader.int()?;
                reader.bytes(size)?;
                let count = reader.int()?;
                let mut leaf = None;
                for index in 0..count {
                    let cert = reader.certificate(version)?;
                    if index == 0 {
                        leaf = Some(cert);
                    }
                }
                leaf
            }
            2 => Some(reader.certificate(version)?),
            3 if jceks => {
                serialized::skip(&mut reader)?;
                None
            }
            _ => return Err(()),
        };
        entries.insert(alias, cert);
    }
    if !password.is_empty() {
        let mut digest = Hasher::new(MessageDigest::sha1()).map_err(|_| ())?;
        for unit in password.encode_utf16() {
            digest.update(&unit.to_be_bytes()).map_err(|_| ())?;
        }
        digest.update(b"Mighty Aphrodite").map_err(|_| ())?;
        digest.update(&bytes[..reader.1]).map_err(|_| ())?;
        let expected = digest.finish().map_err(|_| ())?;
        if reader.bytes(20)? != expected.as_ref() {
            return Err(());
        }
    }
    Ok(entries.into_values().flatten().collect())
}

// With a null password JSSE skips encrypted bags and MAC verification.
fn der<'a>(input: &mut &'a [u8]) -> Result<(u8, &'a [u8]), ()> {
    let tag = *input.first().ok_or(())?;
    let initial = *input.get(1).ok_or(())?;
    let (mut size, mut header) = (usize::from(initial), 2usize);
    if initial & 128 != 0 {
        let width = usize::from(initial & 127);
        if width == 0 || width > std::mem::size_of::<usize>() {
            return Err(());
        }
        size = 0;
        for byte in input.get(2..2 + width).ok_or(())? {
            size = size
                .checked_mul(256)
                .and_then(|v| v.checked_add(usize::from(*byte)))
                .ok_or(())?;
        }
        header += width;
    }
    let end = header.checked_add(size).ok_or(())?;
    let body = input.get(header..end).ok_or(())?;
    *input = input.get(end..).ok_or(())?;
    Ok((tag, body))
}

unsafe extern "C" {
    fn d2i_X509_ALGOR(
        value: *mut *mut openssl_sys::X509_ALGOR,
        input: *mut *const u8,
        length: std::ffi::c_long,
    ) -> *mut openssl_sys::X509_ALGOR;
    fn PKCS12_pbe_crypt(
        algorithm: *const openssl_sys::X509_ALGOR,
        password: *const std::ffi::c_char,
        password_length: std::ffi::c_int,
        input: *const u8,
        input_length: std::ffi::c_int,
        output: *mut *mut u8,
        output_length: *mut std::ffi::c_int,
        encrypt: std::ffi::c_int,
    ) -> *mut u8;
    fn PKCS12_verify_mac(
        archive: *mut openssl_sys::PKCS12,
        password: *const std::ffi::c_char,
        password_length: std::ffi::c_int,
    ) -> std::ffi::c_int;
}

fn decrypt(algorithm: &[u8], bytes: &[u8], password: &str) -> Result<Vec<u8>, ()> {
    // JSSE's legacy PKCS12 PBE includes RC2-40. OpenSSL 3 keeps that cipher
    // in its legacy provider; retain both provider handles for every call.
    static PROVIDERS: OnceLock<Vec<openssl::provider::Provider>> = OnceLock::new();
    PROVIDERS.get_or_init(|| {
        ["default", "legacy"]
            .iter()
            .filter_map(|name| openssl::provider::Provider::load(None, name).ok())
            .collect()
    });
    let length = std::ffi::c_int::try_from(bytes.len()).map_err(|_| ())?;
    let password_length = std::ffi::c_int::try_from(password.len()).map_err(|_| ())?;
    let algorithm_length = std::ffi::c_long::try_from(algorithm.len()).map_err(|_| ())?;
    let (mut output, mut output_length) = (std::ptr::null_mut(), 0);
    // OpenSSL owns the decoded algorithm and allocated plaintext. Both are
    // released on every path; only checked byte lengths cross the FFI boundary.
    unsafe {
        let value = d2i_X509_ALGOR(
            std::ptr::null_mut(),
            &mut algorithm.as_ptr(),
            algorithm_length,
        );
        if value.is_null() {
            return Err(());
        }
        let result = PKCS12_pbe_crypt(
            value,
            password.as_ptr().cast(),
            password_length,
            bytes.as_ptr(),
            length,
            &mut output,
            &mut output_length,
            0,
        );
        openssl_sys::X509_ALGOR_free(value);
        let decoded = if result.is_null() || output.is_null() || output_length < 0 {
            Err(())
        } else {
            Ok(std::slice::from_raw_parts(output, output_length as usize).to_vec())
        };
        openssl_sys::OPENSSL_free(output.cast());
        decoded
    }
}

fn pkcs12(bytes: &[u8], password: &str) -> Result<Vec<Vec<u8>>, ()> {
    let data_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 7, 1];
    let cert_bag_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 0x0c, 0x0a, 1, 3];
    let x509_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 9, 0x16, 1];
    let trusted_oid = [
        0x60, 0x86, 0x48, 1, 0x86, 0xf9, 0x66, 0xad, 0xca, 0x7b, 1, 1,
    ];
    let key_id_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 9, 0x15];
    let (_, mut pfx) = der(&mut &*bytes)?;
    if der(&mut pfx)? != (2, &[3][..]) {
        return Err(());
    }
    let (_, mut auth) = der(&mut pfx)?;
    if der(&mut auth)? != (6, &data_oid[..]) {
        return Err(());
    }
    let (_, mut explicit) = der(&mut auth)?;
    let (_, octets) = der(&mut explicit)?;
    if !password.is_empty() && !pfx.is_empty() {
        let archive = Pkcs12::from_der(bytes).map_err(|_| ())?;
        let length = std::ffi::c_int::try_from(password.len()).map_err(|_| ())?;
        let valid =
            unsafe { PKCS12_verify_mac(archive.as_ptr(), password.as_ptr().cast(), length) };
        if valid != 1 {
            return Err(());
        }
    }
    let (_, mut safes) = der(&mut &*octets)?;
    let mut result = Vec::new();
    let mut keys = BTreeSet::new();
    while !safes.is_empty() {
        let (_, mut info) = der(&mut safes)?;
        let content_type = der(&mut info)?;
        let (_, mut explicit) = der(&mut info)?;
        let decoded;
        let bytes = if content_type == (6, &data_oid[..]) {
            der(&mut explicit)?.1
        } else if content_type == (6, &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 7, 6][..]) {
            if password.is_empty() {
                continue;
            }
            let (_, mut encrypted) = der(&mut explicit)?;
            der(&mut encrypted)?;
            let (_, mut content) = der(&mut encrypted)?;
            if der(&mut content)? != (6, &data_oid[..]) {
                return Err(());
            }
            let before = content;
            der(&mut content)?;
            let algorithm = &before[..before.len() - content.len()];
            let (_, encrypted) = der(&mut content)?;
            decoded = decrypt(algorithm, encrypted, password)?;
            &decoded
        } else {
            return Err(());
        };
        let (_, mut bags) = der(&mut &*bytes)?;
        while !bags.is_empty() {
            let (_, mut bag) = der(&mut bags)?;
            let kind = der(&mut bag)?;
            let (_, mut value) = der(&mut bag)?;
            let mut trusted = false;
            let mut key_id = None;
            if !bag.is_empty() {
                let (_, mut attrs) = der(&mut bag)?;
                while !attrs.is_empty() {
                    let (_, mut attr) = der(&mut attrs)?;
                    let oid = der(&mut attr)?;
                    trusted |= oid == (6, &trusted_oid[..]);
                    if oid == (6, &key_id_oid[..]) {
                        let (_, mut set) = der(&mut attr)?;
                        key_id = Some(der(&mut set)?.1.to_vec());
                    }
                }
            }
            if kind
                == (
                    6,
                    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 0x0c, 0x0a, 1, 1][..],
                )
                || kind
                    == (
                        6,
                        &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 0x0c, 0x0a, 1, 2][..],
                    )
            {
                if let Some(id) = &key_id {
                    keys.insert(id.clone());
                }
            }
            if kind != (6, &cert_bag_oid[..]) {
                continue;
            }
            let (_, mut cert_bag) = der(&mut value)?;
            if der(&mut cert_bag)? != (6, &x509_oid[..]) {
                return Err(());
            }
            let (_, mut explicit) = der(&mut cert_bag)?;
            let (_, cert) = der(&mut explicit)?;
            result.push((
                X509::from_der(cert)
                    .and_then(|v| v.to_der())
                    .map_err(|_| ())?,
                trusted,
                key_id,
            ));
        }
    }
    Ok(result
        .into_iter()
        .filter_map(|(cert, trusted, id)| {
            (trusted || id.as_ref().is_some_and(|id| keys.contains(id))).then_some(cert)
        })
        .collect())
}

pub(super) fn certificates() -> Result<Vec<Vec<u8>>, ()> {
    let property = crate::process_properties::property;
    let kind = property("javax.net.ssl.trustStoreType").unwrap_or_else(|| "pkcs12".into());
    let provider = property("javax.net.ssl.trustStoreProvider").unwrap_or_default();
    let compatible_provider = match kind.to_ascii_lowercase().as_str() {
        "jks" | "caseexactjks" => provider.is_empty() || provider == "SUN",
        "pkcs12" => provider.is_empty() || provider == "SUN" || provider == "SunJSSE",
        "jceks" => provider.is_empty() || provider == "SunJCE",
        _ => false,
    };
    if !compatible_provider {
        return Err(());
    }
    if property("javax.net.ssl.trustStore").as_deref() == Some("NONE") {
        return Ok(Vec::new());
    }
    let java_home = property("java.home").or_else(|| std::env::var("JAVA_HOME").ok());
    let directory = java_home.map(|home| PathBuf::from(home).join("lib/security"));
    let selected = property("javax.net.ssl.trustStore")
        .map(PathBuf::from)
        .or_else(|| directory.as_ref().map(|dir| dir.join("jssecacerts")));
    let bytes = selected
        .and_then(|path| std::fs::read(path).ok())
        .or_else(|| {
            directory
                .as_ref()
                .and_then(|dir| std::fs::read(dir.join("cacerts")).ok())
        });
    let password = property("javax.net.ssl.trustStorePassword").unwrap_or_default();
    let certificates = if let Some(bytes) = bytes {
        if kind.eq_ignore_ascii_case("jceks") {
            jks(&bytes, &password, true)?
        } else if bytes.starts_with(&[0xfe, 0xed, 0xfe, 0xed]) {
            jks(&bytes, &password, false)?
        } else if kind.eq_ignore_ascii_case("caseexactjks") {
            return Err(());
        } else {
            pkcs12(&bytes, &password)?
        }
    } else {
        // The pinned default cacerts is an unencrypted PKCS12 store. The JKS
        // delegator reads it, while strict CaseExactJKS/JCEKS reject its format.
        if kind.eq_ignore_ascii_case("caseexactjks") || kind.eq_ignore_ascii_case("jceks") {
            return Err(());
        }
        let pinned: serde_json::Value =
            serde_json::from_str(include_str!("../data/java-trust-roots-25.json")).unwrap();
        pinned["certificates_der_hex"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| super::unhex(v.as_str().unwrap()))
            .collect()
    };
    Ok(certificates
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

pub(super) fn config() -> Result<ClientConfig, ()> {
    static CONFIG: OnceLock<Result<ClientConfig, ()>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let mut roots = RootCertStore::empty();
            for cert in certificates()? {
                roots.add(CertificateDer::from(cert)).map_err(|_| ())?;
            }
            let identities = keys::load()?;
            let resolver = keys::Resolver::new(&identities)?;
            Ok(ClientConfig::builder()
                .with_root_certificates(roots)
                .with_client_cert_resolver(std::sync::Arc::new(resolver)))
        })
        .clone()
}

#[cfg(feature = "parity-probes")]
pub(super) fn key_report() -> Result<serde_json::Value, ()> {
    config()?;
    keys::load().map(|identities| keys::report(&identities))
}
