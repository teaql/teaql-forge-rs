//! JSSE's default client key manager and native TLS signing identities.
use super::{Reader, der};
use openssl::{
    hash::MessageDigest,
    pkey::{Id, PKey, Private},
    symm::{Cipher, decrypt},
    x509::X509,
};
use rustls::{
    SignatureAlgorithm, SignatureScheme,
    client::ResolvesClientCert,
    sign::{CertifiedKey, Signer, SigningKey},
};
use std::{collections::BTreeMap, sync::Arc};

pub(super) struct Identity {
    pub alias: String,
    pub algorithm: String,
    pub pkcs8: Vec<u8>,
    pub chain: Vec<Vec<u8>>,
}
impl std::fmt::Debug for Identity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Identity")
            .field("alias", &self.alias)
            .field("algorithm", &self.algorithm)
            .field("chain_length", &self.chain.len())
            .finish_non_exhaustive()
    }
}

struct Entry {
    alias: String,
    chain: Vec<Vec<u8>>,
    protected: Vec<u8>,
    secret: Option<super::serialized::Sealed>,
    is_private: bool,
}

fn algorithm_name(id: Id) -> &'static str {
    match id {
        Id::RSA => "RSA",
        Id::RSA_PSS => "RSASSA-PSS",
        Id::EC => "EC",
        Id::DSA => "DSA",
        Id::ED25519 | Id::ED448 => "EdDSA",
        _ => "",
    }
}

fn digest(kind: MessageDigest, parts: &[&[u8]]) -> Result<Vec<u8>, ()> {
    let mut value = openssl::hash::Hasher::new(kind).map_err(|_| ())?;
    for part in parts {
        value.update(part).map_err(|_| ())?;
    }
    value.finish().map(|v| v.to_vec()).map_err(|_| ())
}

fn jce_pbe(parameters: &[u8], encrypted: &[u8], password: &str) -> Result<Vec<u8>, ()> {
    if !password.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err(());
    }
    let (_, mut parameters) = der(&mut &*parameters)?;
    let (_, salt) = der(&mut parameters)?;
    if salt.len() != 8 {
        return Err(());
    }
    let (_, count) = der(&mut parameters)?;
    let count = count
        .iter()
        .try_fold(0usize, |value, b| {
            value.checked_mul(256)?.checked_add(usize::from(*b))
        })
        .ok_or(())?;
    if count == 0 || count > 5_000_000 {
        return Err(());
    }
    let mut salt = salt.to_vec();
    if salt[..4] == salt[4..] {
        salt[..4].reverse();
    }
    let mut key_iv = Vec::new();
    for half in salt.chunks_exact(4) {
        let mut value = digest(MessageDigest::md5(), &[half, password.as_bytes()])?;
        for _ in 1..count {
            value = digest(MessageDigest::md5(), &[&value, password.as_bytes()])?;
        }
        key_iv.extend(value);
    }
    decrypt(
        Cipher::des_ede3_cbc(),
        &key_iv[..24],
        Some(&key_iv[24..]),
        encrypted,
    )
    .map_err(|_| ())
}

fn private_key(protected: &[u8], password: &str, jceks: bool) -> Result<Vec<u8>, ()> {
    let (_, mut info) = der(&mut &*protected)?;
    let algorithm_before = info;
    let (_, mut algorithm) = der(&mut info)?;
    let algorithm_encoded = &algorithm_before[..algorithm_before.len() - info.len()];
    let (_, oid) = der(&mut algorithm)?;
    let (_, encrypted) = der(&mut info)?;
    if oid == [0x2b, 6, 1, 4, 1, 0x2a, 2, 0x11, 1, 1] {
        if password.is_empty() {
            return Err(());
        }
        if encrypted.len() < 40 {
            return Err(());
        }
        let password = password
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>();
        let content = &encrypted[20..encrypted.len() - 20];
        let mut previous = encrypted[..20].to_vec();
        let mut plain = Vec::with_capacity(content.len());
        for chunk in content.chunks(20) {
            previous = digest(MessageDigest::sha1(), &[&password, &previous])?;
            plain.extend(chunk.iter().zip(&previous).map(|(a, b)| a ^ b));
        }
        if digest(MessageDigest::sha1(), &[&password, &plain])? != encrypted[encrypted.len() - 20..]
        {
            return Err(());
        }
        Ok(plain)
    } else if jceks && oid == [0x2b, 6, 1, 4, 1, 0x2a, 2, 0x13, 1] {
        if password.is_empty() {
            return Err(());
        }
        jce_pbe(algorithm, encrypted, password)
    } else {
        super::decrypt(algorithm_encoded, encrypted, password)
    }
}

fn jks(bytes: &[u8], password: &str, jceks: bool) -> Result<Vec<Entry>, ()> {
    super::jks(bytes, password, jceks)?;
    let mut reader = Reader(bytes, 0);
    reader.bytes(4)?;
    let version = reader.int()?;
    let count = reader.int()?;
    let mut entries = Vec::<Entry>::new();
    for _ in 0..count {
        let tag = reader.int()?;
        let alias = String::from_utf16(&reader.utf()?).map_err(|_| ())?;
        reader.bytes(8)?;
        let mut entry = Entry {
            alias,
            chain: Vec::new(),
            protected: Vec::new(),
            secret: None,
            is_private: tag == 1,
        };
        match tag {
            1 => {
                let size = reader.int()?;
                entry.protected = reader.bytes(size)?.to_vec();
                let count = reader.int()?;
                for _ in 0..count {
                    entry.chain.push(reader.certificate(version)?);
                }
            }
            2 => {
                reader.certificate(version)?;
            }
            3 => {
                entry.secret = Some(super::serialized::read(&mut reader)?);
            }
            _ => return Err(()),
        }
        if let Some(index) = entries.iter().position(|old| old.alias == entry.alias) {
            entries[index] = entry;
        } else {
            entries.push(entry);
        }
    }
    // Hashtable iteration is descending buckets, with insertion at each head.
    let raw_hash = |value: &str| {
        value
            .encode_utf16()
            .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(u32::from(c)))
            & 0x7fff_ffff
    };
    let mut bins = vec![Vec::<usize>::new(); 11];
    for (index, entry) in entries.iter().enumerate() {
        if index >= bins.len() * 3 / 4 {
            let mut resized = vec![Vec::new(); bins.len() * 2 + 1];
            for bucket in bins.iter().rev() {
                for &id in bucket {
                    let n = raw_hash(&entries[id].alias) as usize % resized.len();
                    resized[n].insert(0, id);
                }
            }
            bins = resized;
        }
        let bucket = raw_hash(&entry.alias) as usize % bins.len();
        bins[bucket].insert(0, index);
    }
    let mut source = entries.into_iter().map(Some).collect::<Vec<_>>();
    Ok(bins
        .into_iter()
        .rev()
        .flatten()
        .map(|id| source[id].take().unwrap())
        .collect())
}

fn identity(entry: Entry, password: &str, jceks: bool) -> Result<Option<Identity>, ()> {
    if let Some(secret) = entry.secret {
        if password.is_empty() {
            return Err(());
        }
        let plain = jce_pbe(&secret.parameters, &secret.encrypted, password)?;
        if !plain.starts_with(&[0xac, 0xed, 0, 5]) {
            return Err(());
        }
        return Ok(None);
    }
    if entry.protected.is_empty() {
        return Ok(None);
    }
    let pkcs8 = private_key(&entry.protected, password, jceks)?;
    if !entry.is_private {
        let (tag, mut info) = der(&mut &*pkcs8)?;
        if tag != 0x30 || der(&mut info)?.0 != 2 {
            return Err(());
        }
        let (tag, mut algorithm) = der(&mut info)?;
        if tag != 0x30 || der(&mut algorithm)?.0 != 6 {
            return Err(());
        }
        let (tag, key) = der(&mut info)?;
        if tag != 4 || key.is_empty() {
            return Err(());
        }
        return Ok(None);
    }
    let key = PKey::private_key_from_pkcs8(&pkcs8).map_err(|_| ())?;
    if entry.chain.is_empty() {
        return Ok(None);
    }
    let algorithm = algorithm_name(key.id()).to_string();
    Ok(Some(Identity {
        alias: entry.alias,
        algorithm,
        pkcs8,
        chain: entry.chain,
    }))
}

pub(super) fn load() -> Result<Vec<Identity>, ()> {
    let property = crate::process_properties::property;
    let path = property("javax.net.ssl.keyStore").unwrap_or_default();
    let kind = property("javax.net.ssl.keyStoreType").unwrap_or_else(|| "pkcs12".into());
    let provider = property("javax.net.ssl.keyStoreProvider").unwrap_or_default();
    let password = property("javax.net.ssl.keyStorePassword").unwrap_or_default();
    let bytes = if path.is_empty() || path == "NONE" {
        None
    } else {
        Some(std::fs::read(&path).map_err(|_| ())?)
    };
    if kind.is_empty() {
        return Ok(Vec::new());
    }
    let valid = match kind.to_ascii_lowercase().as_str() {
        "jks" | "caseexactjks" => provider.is_empty() || provider == "SUN",
        "jceks" => provider.is_empty() || provider == "SunJCE",
        "pkcs12" => provider.is_empty() || provider == "SUN" || provider == "SunJSSE",
        _ => false,
    };
    if !valid {
        return Err(());
    }
    let Some(bytes) = bytes else {
        return Ok(Vec::new());
    };
    let jceks = kind.eq_ignore_ascii_case("jceks");
    let entries = if jceks || bytes.starts_with(&[0xfe, 0xed, 0xfe, 0xed]) {
        jks(&bytes, &password, jceks)?
    } else if kind.eq_ignore_ascii_case("caseexactjks") {
        return Err(());
    } else {
        pkcs12(&bytes, &password)?
    };
    let mut identities = BTreeMap::new();
    let mut insertion = Vec::new();
    for entry in entries {
        if let Some(identity) = identity(entry, &password, jceks)? {
            if !identities.contains_key(&identity.alias) {
                insertion.push(identity.alias.clone());
            }
            identities.insert(identity.alias.clone(), identity);
        }
    }
    Ok(crate::java_hash_set::attribute_names(insertion)
        .into_iter()
        .map(|alias| identities.remove(&alias).unwrap())
        .collect())
}

fn pkcs12(bytes: &[u8], password: &str) -> Result<Vec<Entry>, ()> {
    super::pkcs12(bytes, password)?;
    let data_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 7, 1];
    let key_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 0x0c, 0x0a, 1, 2];
    let cert_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 0x0c, 0x0a, 1, 3];
    let secret_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 0x0c, 0x0a, 1, 5];
    let friendly_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 9, 0x14];
    let id_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 9, 0x15];
    let trusted_oid = [
        0x60, 0x86, 0x48, 1, 0x86, 0xf9, 0x66, 0xad, 0xca, 0x7b, 1, 1,
    ];
    let (_, mut pfx) = der(&mut &*bytes)?;
    der(&mut pfx)?;
    let (_, mut auth) = der(&mut pfx)?;
    der(&mut auth)?;
    let (_, mut explicit) = der(&mut auth)?;
    let (_, octets) = der(&mut explicit)?;
    let (_, mut safes) = der(&mut &*octets)?;
    let mut entries = Vec::<(Entry, Option<Vec<u8>>)>::new();
    let mut certificates = Vec::<(Vec<u8>, Option<Vec<u8>>, Option<String>)>::new();
    let (mut private_count, mut generated) = (0, 0);
    while !safes.is_empty() {
        let (_, mut info) = der(&mut safes)?;
        let kind = der(&mut info)?;
        let (_, mut explicit) = der(&mut info)?;
        let decoded;
        let content = if kind == (6, &data_oid[..]) {
            der(&mut explicit)?.1
        } else {
            if password.is_empty() {
                continue;
            }
            let (_, mut encrypted) = der(&mut explicit)?;
            der(&mut encrypted)?;
            let (_, mut info) = der(&mut encrypted)?;
            der(&mut info)?;
            let before = info;
            der(&mut info)?;
            let algorithm = &before[..before.len() - info.len()];
            let (_, ciphertext) = der(&mut info)?;
            decoded = super::decrypt(algorithm, ciphertext, password)?;
            &decoded
        };
        let (_, mut bags) = der(&mut &*content)?;
        let mut bag_index = 0;
        while !bags.is_empty() {
            let (_, mut bag) = der(&mut bags)?;
            let bag_kind = der(&mut bag)?;
            let (_, value) = der(&mut bag)?;
            let (mut alias, mut id, mut trusted) = (None, None, false);
            if !bag.is_empty() {
                let (_, mut attrs) = der(&mut bag)?;
                while !attrs.is_empty() {
                    let (_, mut attr) = der(&mut attrs)?;
                    let kind = der(&mut attr)?;
                    let (_, mut values) = der(&mut attr)?;
                    if kind == (6, &friendly_oid[..]) {
                        let (_, name) = der(&mut values)?;
                        if name.len() % 2 != 0 {
                            return Err(());
                        }
                        alias = Some(
                            String::from_utf16(
                                &name
                                    .chunks_exact(2)
                                    .map(|p| u16::from_be_bytes([p[0], p[1]]))
                                    .collect::<Vec<_>>(),
                            )
                            .map_err(|_| ())?,
                        );
                    } else if kind == (6, &id_oid[..]) {
                        id = Some(der(&mut values)?.1.to_vec());
                    } else if kind == (6, &trusted_oid[..]) {
                        trusted = true;
                    }
                }
            }
            let private = bag_kind == (6, &key_oid[..]);
            let secret = bag_kind == (6, &secret_oid[..]);
            let certificate = bag_kind == (6, &cert_oid[..]);
            if private || secret || certificate && trusted {
                if private {
                    private_count += 1;
                    if id.is_none() {
                        if private_count == 1 {
                            id = Some(b"01".to_vec());
                        } else {
                            continue;
                        }
                    }
                }
                let name = alias
                    .clone()
                    .unwrap_or_else(|| {
                        generated += 1;
                        generated.to_string()
                    })
                    .to_lowercase();
                let protected = if private {
                    value.to_vec()
                } else if secret {
                    let (_, mut bag) = der(&mut &*value)?;
                    der(&mut bag)?;
                    let (_, mut explicit) = der(&mut bag)?;
                    der(&mut explicit)?.1.to_vec()
                } else {
                    Vec::new()
                };
                let entry = Entry {
                    alias: name,
                    chain: Vec::new(),
                    protected,
                    secret: None,
                    is_private: private,
                };
                if let Some(index) = entries.iter().position(|(old, _)| old.alias == entry.alias) {
                    entries[index] = (entry, id.clone());
                } else {
                    entries.push((entry, id.clone()));
                }
            }
            if certificate {
                let (_, mut cert_bag) = der(&mut &*value)?;
                der(&mut cert_bag)?;
                let (_, mut explicit) = der(&mut cert_bag)?;
                let (_, cert) = der(&mut explicit)?;
                let cert = X509::from_der(cert)
                    .and_then(|v| v.to_der())
                    .map_err(|_| ())?;
                if id.is_none() && private_count == 1 && bag_index == 0 {
                    id = Some(b"01".to_vec());
                }
                certificates.push((cert, id, alias));
            }
            bag_index += 1;
        }
    }
    for (entry, id) in &mut entries {
        if !entry.is_private || id.is_none() {
            continue;
        }
        let mut key_match = None;
        let mut alias_match = None;
        for (index, (_, cert_id, alias)) in certificates.iter().enumerate() {
            let alias_equal = alias
                .as_ref()
                .is_some_and(|name| name.eq_ignore_ascii_case(&entry.alias));
            if cert_id == id {
                key_match = Some(index);
                if alias_equal {
                    break;
                }
            } else if alias_equal {
                alias_match = Some(index);
            }
        }
        let mut current = key_match.or(alias_match);
        while let Some(index) = current {
            let bytes = &certificates[index].0;
            if entry.chain.contains(bytes) {
                break;
            }
            entry.chain.push(bytes.clone());
            let cert = X509::from_der(bytes).map_err(|_| ())?;
            let self_issued = cert
                .subject_name()
                .try_cmp(cert.issuer_name())
                .map_err(|_| ())?
                .is_eq();
            if self_issued
                && cert
                    .public_key()
                    .and_then(|key| cert.verify(&key))
                    .unwrap_or(false)
            {
                break;
            }
            let mut fallback = None;
            current = None;
            for (index, (candidate, _, _)) in certificates.iter().enumerate() {
                let candidate = X509::from_der(candidate).map_err(|_| ())?;
                if cert
                    .issuer_name()
                    .try_cmp(candidate.subject_name())
                    .map_err(|_| ())?
                    .is_eq()
                {
                    fallback = Some(index);
                    if cert
                        .authority_key_id()
                        .zip(candidate.subject_key_id())
                        .is_some_and(|(a, b)| a.as_slice() == b.as_slice())
                    {
                        current = Some(index);
                        break;
                    }
                }
            }
            current = current.or(fallback);
        }
    }
    Ok(entries.into_iter().map(|(entry, _)| entry).collect())
}

#[derive(Debug)]
pub(super) struct Resolver {
    identities: Vec<(Vec<Vec<u8>>, Arc<CertifiedKey>, String, bool)>,
}

#[derive(Debug)]
struct Key {
    key: PKey<Private>,
    spki: Vec<u8>,
}

impl SigningKey for Key {
    fn choose_scheme(&self, offered: &[SignatureScheme]) -> Option<Box<dyn Signer>> {
        offered
            .iter()
            .copied()
            .find(|scheme| {
                let value = u16::from(*scheme);
                match self.key.id() {
                    Id::RSA => matches!(
                        value,
                        0x0201 | 0x0401 | 0x0501 | 0x0601 | 0x0804 | 0x0805 | 0x0806
                    ),
                    Id::RSA_PSS => matches!(value, 0x0809 | 0x080a | 0x080b),
                    Id::EC => self
                        .key
                        .ec_key()
                        .ok()
                        .and_then(|key| key.group().curve_name())
                        .is_some_and(|curve| {
                            (curve == openssl::nid::Nid::X9_62_PRIME256V1 && value == 0x0403)
                                || (curve == openssl::nid::Nid::SECP384R1 && value == 0x0503)
                                || (curve == openssl::nid::Nid::SECP521R1 && value == 0x0603)
                                || value == 0x0203
                        }),
                    Id::DSA => matches!(value, 0x0202 | 0x0402),
                    Id::ED25519 => value == 0x0807,
                    Id::ED448 => value == 0x0808,
                    _ => false,
                }
            })
            .map(|scheme| {
                Box::new(OpenSslSigner {
                    key: self.key.clone(),
                    scheme,
                }) as Box<dyn Signer>
            })
    }
    fn algorithm(&self) -> SignatureAlgorithm {
        match self.key.id() {
            Id::RSA | Id::RSA_PSS => SignatureAlgorithm::RSA,
            Id::EC => SignatureAlgorithm::ECDSA,
            Id::DSA => SignatureAlgorithm::DSA,
            Id::ED25519 => SignatureAlgorithm::ED25519,
            Id::ED448 => SignatureAlgorithm::ED448,
            _ => SignatureAlgorithm::Anonymous,
        }
    }
    fn public_key(&self) -> Option<rustls::pki_types::SubjectPublicKeyInfoDer<'_>> {
        Some(self.spki.as_slice().into())
    }
}

#[derive(Debug)]
struct OpenSslSigner {
    key: PKey<Private>,
    scheme: SignatureScheme,
}
impl Signer for OpenSslSigner {
    fn scheme(&self) -> SignatureScheme {
        self.scheme
    }
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, rustls::Error> {
        let failure = || rustls::Error::General("client certificate signing failed".into());
        let value = u16::from(self.scheme);
        let digest = match value {
            0x0201..=0x0203 => Some(MessageDigest::sha1()),
            0x0401..=0x0403 | 0x0804 | 0x0809 => Some(MessageDigest::sha256()),
            0x0501 | 0x0503 | 0x0805 | 0x080a => Some(MessageDigest::sha384()),
            0x0601 | 0x0603 | 0x0806 | 0x080b => Some(MessageDigest::sha512()),
            _ => None,
        };
        let mut signer = if let Some(digest) = digest {
            openssl::sign::Signer::new(digest, &self.key)
        } else {
            openssl::sign::Signer::new_without_digest(&self.key)
        }
        .map_err(|_| failure())?;
        if matches!(value,0x0804..=0x0806|0x0809..=0x080b) {
            signer
                .set_rsa_padding(openssl::rsa::Padding::PKCS1_PSS)
                .map_err(|_| failure())?;
            signer
                .set_rsa_mgf1_md(digest.unwrap())
                .map_err(|_| failure())?;
            signer
                .set_rsa_pss_saltlen(openssl::sign::RsaPssSaltlen::DIGEST_LENGTH)
                .map_err(|_| failure())?;
        }
        if digest.is_none() {
            signer.sign_oneshot_to_vec(message).map_err(|_| failure())
        } else {
            signer
                .update(message)
                .and_then(|_| signer.sign_to_vec())
                .map_err(|_| failure())
        }
    }
}

impl Resolver {
    pub(super) fn new(identities: &[Identity]) -> Result<Self, ()> {
        let mut keys = Vec::new();
        for identity in identities {
            let key = PKey::private_key_from_pkcs8(&identity.pkcs8).map_err(|_| ())?;
            let spki = key.public_key_to_der().map_err(|_| ())?;
            let signing = Arc::new(Key { key, spki });
            let issuers = identity
                .chain
                .iter()
                .map(|der| {
                    X509::from_der(der)
                        .and_then(|cert| cert.issuer_name().to_der())
                        .map_err(|_| ())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let cert = CertifiedKey::new(
                identity
                    .chain
                    .iter()
                    .cloned()
                    .map(rustls::pki_types::CertificateDer::from)
                    .collect(),
                signing,
            );
            let leaf = X509::from_der(&identity.chain[0])
                .and_then(|cert| cert.public_key())
                .map_err(|_| ())?;
            keys.push((
                issuers,
                Arc::new(cert),
                algorithm_name(leaf.id()).into(),
                algorithm_name(leaf.id()) == identity.algorithm,
            ));
        }
        Ok(Self { identities: keys })
    }
}
impl ResolvesClientCert for Resolver {
    fn resolve(
        &self,
        authorities: &[&[u8]],
        schemes: &[SignatureScheme],
    ) -> Option<Arc<CertifiedKey>> {
        let mut seen = std::collections::BTreeSet::new();
        for scheme in schemes {
            let kind = match u16::from(*scheme) {
                0x0201 | 0x0401 | 0x0501 | 0x0601 | 0x0804..=0x0806 => "RSA",
                0x0203 | 0x0403 | 0x0503 | 0x0603 => "EC",
                0x0202 | 0x0402 => "DSA",
                0x0807 | 0x0808 => "EdDSA",
                0x0809..=0x080b => "RSASSA-PSS",
                _ => continue,
            };
            if !seen.insert(kind) {
                continue;
            }
            if let Some((_, key, _, matched)) =
                self.identities.iter().find(|(issuers, _, algorithm, _)| {
                    algorithm == kind
                        && (authorities.is_empty()
                            || authorities
                                .iter()
                                .any(|name| issuers.iter().any(|issuer| issuer == name)))
                })
            {
                return (*matched && key.key.choose_scheme(schemes).is_some()).then(|| key.clone());
            }
        }
        None
    }
    fn has_certs(&self) -> bool {
        !self.identities.is_empty()
    }
}

#[cfg(feature = "parity-probes")]
pub(super) fn report(identities: &[Identity]) -> serde_json::Value {
    let mut aliases = serde_json::Map::new();
    for identity in identities {
        aliases.insert(identity.alias.clone(),serde_json::json!({"key_algorithm":identity.algorithm,
            "private_key_pkcs8_sha256":super::super::hex(openssl::hash::hash(MessageDigest::sha256(),&identity.pkcs8).unwrap().as_ref()),
            "chain_der_hex":identity.chain.iter().map(|der|super::super::hex(der)).collect::<Vec<_>>()}));
    }
    let matches = |identity: &Identity, query: &str| -> bool {
        let Ok(leaf) = X509::from_der(&identity.chain[0]) else {
            return false;
        };
        let Ok(public) = leaf.public_key() else {
            return false;
        };
        let (kind, signature) = query
            .split_once('_')
            .map_or((query, None), |(kind, signature)| (kind, Some(signature)));
        if kind != algorithm_name(public.id()) {
            return false;
        }
        let Some(signature) = signature else {
            return true;
        };
        if identity.chain.len() > 1 {
            X509::from_der(&identity.chain[1])
                .and_then(|cert| cert.public_key())
                .is_ok_and(|public| algorithm_name(public.id()) == signature)
        } else {
            let signing = leaf
                .signature_algorithm()
                .object()
                .nid()
                .signature_algorithms()
                .map(|value| value.pkey);
            let kind = if signing == Some(openssl::nid::Nid::RSAENCRYPTION) {
                "RSA"
            } else if signing == Some(openssl::nid::Nid::X9_62_ID_ECPUBLICKEY) {
                "ECDSA"
            } else if signing == Some(openssl::nid::Nid::DSA) {
                "DSA"
            } else {
                ""
            };
            !kind.is_empty() && kind.to_uppercase().starts_with(&signature.to_uppercase())
        }
    };
    let mut available = serde_json::Map::new();
    for kind in [
        "RSA",
        "EC",
        "DSA",
        "RSASSA-PSS",
        "EdDSA",
        "Ed25519",
        "Ed448",
        "RSA_RSA",
        "EC_RSA",
        "EC_EC",
    ] {
        let names = identities
            .iter()
            .filter(|identity| matches(identity, kind))
            .map(|id| id.alias.clone())
            .collect::<Vec<_>>();
        available.insert(
            kind.into(),
            if names.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!(names)
            },
        );
    }
    let mut choices = serde_json::Map::new();
    for query in [
        vec!["RSA", "EC"],
        vec!["EC", "RSA"],
        vec!["RSA"],
        vec!["DSA", "RSA"],
        vec!["EdDSA", "RSA"],
        vec!["RSASSA-PSS", "RSA"],
    ] {
        let alias = query.iter().find_map(|kind| {
            identities
                .iter()
                .find(|identity| matches(identity, kind))
                .map(|id| id.alias.clone())
        });
        choices.insert(query.join(","), serde_json::json!(alias));
    }
    serde_json::json!({"aliases":aliases,"client_aliases":available,"choices":choices})
}
