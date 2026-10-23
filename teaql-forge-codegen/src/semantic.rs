//! Stable semantic identities, independent of generated names and languages.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use teaql_forge_model::ir::{Domain, EntityMember};

pub const SIDECAR_PATH: &str = ".teaql/semantic-ids.json";
pub const OUTPUT_PATH: &str = "teaql/semantic-ids.json";

mod entropy;
mod json;
#[cfg(feature = "parity-probes")]
pub use entropy::replay as replay_id_entropy;

pub fn load_document(bytes: &[u8]) -> Result<SemanticIdentityDocument, String> {
    json::load(bytes).map_err(|message| format!("invalid {SIDECAR_PATH}: {message}"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SemanticIdentityDocument {
    pub schema_version: i32,
    pub revision: i64,
    pub model_id: Option<String>,
    #[serde(deserialize_with = "null_vec")]
    pub bindings: Vec<SemanticIdentityBinding>,
}

impl Default for SemanticIdentityDocument {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            model_id: None,
            bindings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SemanticIdentityBinding {
    pub id: Option<String>,
    pub kind: Option<String>,
    pub parent_id: Option<String>,
    pub path: Option<String>,
    #[serde(deserialize_with = "null_vec")]
    pub aliases: Vec<String>,
    pub status: String,
    #[serde(deserialize_with = "null_vec")]
    pub derived_from: Vec<String>,
}

fn null_vec<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Vec<T>, D::Error> {
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

impl Default for SemanticIdentityBinding {
    fn default() -> Self {
        Self {
            id: None,
            kind: None,
            parent_id: None,
            path: None,
            aliases: Vec::new(),
            status: "active".into(),
            derived_from: Vec::new(),
        }
    }
}

fn java_string(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

/// Java String.isBlank uses Character.isWhitespace, which excludes NBSP/NEL.
pub fn is_blank(value: &str) -> bool {
    value.chars().all(|ch| matches!(ch,'\u{9}'..='\u{d}'|'\u{1c}'..='\u{20}'|'\u{1680}'|'\u{2000}'..='\u{2006}'|'\u{2008}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{205f}'|'\u{3000}'))
}

fn jackson_escapes(bytes: Vec<u8>) -> Vec<u8> {
    let mut result = Vec::with_capacity(bytes.len());
    let mut position = 0;
    while position < bytes.len() {
        if bytes[position] >= 0xf0 {
            let ch = std::str::from_utf8(&bytes[position..position + 4])
                .expect("JSON UTF-8")
                .chars()
                .next()
                .unwrap() as u32
                - 0x10000;
            result.extend_from_slice(
                format!(
                    "\\u{:04X}\\u{:04X}",
                    0xd800 + (ch >> 10),
                    0xdc00 + (ch & 0x3ff)
                )
                .as_bytes(),
            );
            position += 4;
        } else if bytes[position] == b'\\' && position + 1 < bytes.len() {
            let end = if bytes[position + 1] == b'u' && position + 6 <= bytes.len() {
                position + 6
            } else {
                position + 2
            };
            let start = result.len();
            result.extend_from_slice(&bytes[position..end]);
            if end == position + 6 {
                result[start + 2..start + 6].make_ascii_uppercase();
            }
            position = end;
        } else {
            result.push(bytes[position]);
            position += 1;
        }
    }
    result
}

/// Jackson UTF-8 output uses uppercase escapes and supplementary surrogate pairs.
pub fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(value).map(jackson_escapes)
}

// java.util.UUID.fromString accepts abbreviated hexadecimal groups.
fn valid_id(id: Option<&str>, kind: Option<&str>) -> bool {
    let (Some(id), Some(kind)) = (id, kind) else {
        return false;
    };
    let Some(value) = id.strip_prefix(&format!("sem:{kind}:")) else {
        return false;
    };
    if value != value.to_lowercase() || value.len() > 36 {
        return false;
    }
    let parts = value.split('-').collect::<Vec<_>>();
    parts.len() == 5
        && parts
            .iter()
            .all(|part| !part.is_empty() && u64::from_str_radix(part, 16).is_ok())
}

fn required_bindings(domain: &Domain) -> Vec<(&str, String, Option<&str>)> {
    let mut result = Vec::new();
    for entity in &domain.entities {
        let kind = if entity
            .metadata
            .get("constant")
            .is_some_and(|value| teaql_forge_model::parser::hutool_true(value))
        {
            "constant"
        } else {
            "entity"
        };
        result.push((kind, entity.name.clone(), None));
        for member in &entity.members {
            let name = match member {
                EntityMember::Field(field) => &field.name,
                EntityMember::Relation(relation) => &relation.name,
            };
            if name != "id" && name != "version" {
                result.push((
                    "field",
                    format!("{}.{name}", entity.name),
                    Some(entity.name.as_str()),
                ));
            }
        }
    }
    result
}

pub fn validate(document: &SemanticIdentityDocument, domain: Option<&Domain>) -> Vec<String> {
    let mut errors = Vec::new();
    if document.schema_version != 1 {
        errors.push("schemaVersion must be 1".into());
    }
    if !valid_id(document.model_id.as_deref(), Some("model")) {
        errors.push("modelId is not a valid sem:model ID".into());
    }
    let mut by_id = HashMap::new();
    let mut active_by_path = HashMap::new();
    let mut tombstones = Vec::new();
    let mut aliases = HashSet::new();
    for (index, binding) in document.bindings.iter().enumerate() {
        if !matches!(
            binding.kind.as_deref(),
            Some(
                "entity"
                    | "field"
                    | "constant"
                    | "state_machine"
                    | "transition"
                    | "action"
                    | "constraint"
            )
        ) {
            errors.push(format!(
                "unsupported kind: {}",
                java_string(binding.kind.as_deref())
            ));
        }
        if !valid_id(binding.id.as_deref(), binding.kind.as_deref()) {
            errors.push(format!(
                "ID/kind mismatch: {}",
                java_string(binding.id.as_deref())
            ));
        }
        if by_id.insert(binding.id.as_deref(), index).is_some() {
            errors.push(format!(
                "duplicate ID: {}",
                java_string(binding.id.as_deref())
            ));
        }
        if !matches!(binding.status.as_str(), "active" | "tombstone") {
            errors.push(format!(
                "invalid lifecycle status for {}",
                java_string(binding.id.as_deref())
            ));
        }
        if binding.status == "active"
            && active_by_path
                .insert(binding.path.as_deref(), index)
                .is_some()
        {
            errors.push(format!(
                "duplicate active path: {}",
                java_string(binding.path.as_deref())
            ));
        }
        if binding.status == "tombstone" && !tombstones.contains(&binding.path.as_deref()) {
            tombstones.push(binding.path.as_deref());
        }
        for alias in &binding.aliases {
            if !aliases.insert(alias) {
                errors.push(format!("ambiguous alias: {alias}"));
            }
        }
    }
    for (index, binding) in document.bindings.iter().enumerate() {
        let id = java_string(binding.id.as_deref());
        let known =
            |id: &str| by_id.contains_key(&Some(id)) || document.model_id.as_deref() == Some(id);
        if let Some(parent) = &binding.parent_id {
            if !known(parent) {
                errors.push(format!("unknown parentId for {id}: {parent}"));
            }
        }
        for source in &binding.derived_from {
            if !known(source) {
                errors.push(format!("unknown derivedFrom ID: {source}"));
            }
        }
        if binding.kind.as_deref() == Some("field")
            && !by_id
                .get(&binding.parent_id.as_deref())
                .is_some_and(|parent| {
                    matches!(
                        document.bindings[*parent].kind.as_deref(),
                        Some("entity" | "constant")
                    )
                })
        {
            errors.push(format!(
                "field must have an entity or constant parent: {id}"
            ));
        }
        for alias in &binding.aliases {
            if active_by_path
                .get(&Some(alias.as_str()))
                .is_some_and(|current| *current != index)
            {
                errors.push(format!("alias collides with active path: {alias}"));
            }
        }
    }
    for path in tombstones {
        if active_by_path.contains_key(&path) {
            errors.push(format!(
                "tombstone path was reused without restore: {}",
                java_string(path)
            ));
        }
    }
    if let Some(domain) = domain {
        for (kind, path, parent_path) in required_bindings(domain) {
            let Some(index) = active_by_path.get(&Some(path.as_str())) else {
                errors.push(format!("MISSING_SEMANTIC_ID: {path}"));
                continue;
            };
            let binding = &document.bindings[*index];
            if binding.kind.as_deref() != Some(kind) {
                errors.push(format!(
                    "semantic kind mismatch for {path}: expected={kind}, actual={}",
                    java_string(binding.kind.as_deref())
                ));
            }
            if let Some(parent_path) = parent_path {
                if !active_by_path
                    .get(&Some(parent_path))
                    .is_some_and(|parent| document.bindings[*parent].id == binding.parent_id)
                {
                    errors.push(format!("semantic parent mismatch for {path}"));
                }
            }
        }
    }
    errors
}

fn new_id(kind: &str) -> Result<String, String> {
    let mut bytes = [0u8; 16];
    entropy::fill(&mut bytes)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "sem:{kind}:{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

fn allocate(
    kind: &str,
    path: String,
    parent_id: Option<String>,
) -> Result<SemanticIdentityBinding, String> {
    Ok(SemanticIdentityBinding {
        id: Some(new_id(kind)?),
        kind: Some(kind.into()),
        path: Some(path),
        parent_id,
        ..Default::default()
    })
}

pub fn initialize(domain: &Domain) -> Result<SemanticIdentityDocument, String> {
    let mut document = SemanticIdentityDocument {
        revision: 1,
        model_id: Some(new_id("model")?),
        ..Default::default()
    };
    for (kind, path, parent_path) in required_bindings(domain) {
        let parent_id = parent_path
            .and_then(|parent| {
                document
                    .bindings
                    .iter()
                    .find(|binding| binding.path.as_deref() == Some(parent))
            })
            .and_then(|binding| binding.id.clone());
        document.bindings.push(allocate(kind, path, parent_id)?);
    }
    Ok(document)
}

fn check_revision(document: &SemanticIdentityDocument, expected: i64) -> Result<(), String> {
    if document.revision == expected {
        Ok(())
    } else {
        Err(format!(
            "semantic identity revision conflict: expected={expected}, actual={}",
            document.revision
        ))
    }
}

pub fn reconcile(
    document: &mut SemanticIdentityDocument,
    expected: i64,
    domain: &Domain,
) -> Result<(), String> {
    check_revision(document, expected)?;
    let errors = validate(document, None);
    if !errors.is_empty() {
        return Err(format!(
            "invalid semantic identity document: {}",
            errors.join("; ")
        ));
    }
    let mut changed = false;
    for (kind, path, parent_path) in required_bindings(domain) {
        if document
            .bindings
            .iter()
            .any(|binding| binding.status == "active" && binding.path.as_deref() == Some(&path))
        {
            continue;
        }
        let parent_id = match parent_path {
            Some(parent) => Some(
                document
                    .bindings
                    .iter()
                    .find(|binding| {
                        binding.status == "active" && binding.path.as_deref() == Some(parent)
                    })
                    .and_then(|binding| binding.id.clone())
                    .ok_or_else(|| {
                        format!("cannot allocate semantic child before parent: {path}")
                    })?,
            ),
            None => None,
        };
        document.bindings.push(allocate(kind, path, parent_id)?);
        changed = true;
    }
    if changed {
        document.revision = expected.wrapping_add(1);
    }
    Ok(())
}

pub fn mutate(
    document: &mut SemanticIdentityDocument,
    expected: i64,
    id: &str,
    operation: &str,
    new_path: Option<&str>,
) -> Result<(), String> {
    check_revision(document, expected)?;
    let index = document
        .bindings
        .iter()
        .position(|binding| binding.id.as_deref() == Some(id))
        .ok_or_else(|| format!("unknown semantic ID: {id}"))?;
    let current = &document.bindings[index];
    let path = match operation {
        "rename" => {
            if current.status != "active" {
                return Err(format!("cannot rename tombstone: {id}"));
            }
            let path = new_path
                .filter(|path| !is_blank(path))
                .ok_or("new path must not be blank")?;
            Some(path)
        }
        "restore" => current.path.as_deref(),
        "delete" => None,
        _ => return Err(format!("unknown semantic identity operation: {operation}")),
    };
    if operation != "delete"
        && document
            .bindings
            .iter()
            .enumerate()
            .any(|(other, binding)| {
                other != index && binding.status == "active" && binding.path.as_deref() == path
            })
    {
        return Err(format!("active path already exists: {}", java_string(path)));
    }
    let binding = &mut document.bindings[index];
    let changed = match operation {
        "rename" if binding.path.as_deref() != new_path => {
            if let Some(old_path) = &binding.path {
                if !binding.aliases.contains(old_path) {
                    binding.aliases.push(old_path.clone());
                }
            }
            binding.path = new_path.map(str::to_owned);
            true
        }
        "delete" if binding.status != "tombstone" => {
            binding.status = "tombstone".into();
            true
        }
        "restore" if binding.status != "active" => {
            binding.status = "active".into();
            true
        }
        _ => false,
    };
    if changed {
        document.revision = expected.wrapping_add(1);
    }
    Ok(())
}

/// Jackson's DefaultPrettyPrinter uses spaced colons and inline arrays of scalars.
pub fn write_string(document: &SemanticIdentityDocument) -> String {
    fn format(value: &serde_json::Value, depth: usize) -> String {
        match value {
            serde_json::Value::Object(entries) => {
                // Property order comes from the Java document and binding declarations.
                let order: &[&str] = if entries.contains_key("schemaVersion") {
                    &["schemaVersion", "revision", "modelId", "bindings"]
                } else {
                    &[
                        "id",
                        "kind",
                        "parentId",
                        "path",
                        "aliases",
                        "status",
                        "derivedFrom",
                    ]
                };
                let indent = "  ".repeat(depth + 1);
                let fields = order
                    .iter()
                    .filter_map(|key| {
                        entries.get(*key).map(|value| {
                            format!("{indent}\"{key}\" : {}", format(value, depth + 1))
                        })
                    })
                    .collect::<Vec<_>>();
                format!("{{\n{}\n{}}}", fields.join(",\n"), "  ".repeat(depth))
            }
            serde_json::Value::Array(values) => {
                if values.is_empty() {
                    "[ ]".into()
                } else {
                    format!(
                        "[ {} ]",
                        values
                            .iter()
                            .map(|value| format(value, depth))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            _ => value.to_string(),
        }
    }
    let formatted = format(
        &serde_json::to_value(document).expect("serializable identity document"),
        0,
    );
    String::from_utf8(jackson_escapes(formatted.into_bytes())).expect("JSON UTF-8")
}
