//! HTTP DTOs follow Boot's Jackson configuration, including nullable members.
use axum::http::StatusCode;
mod binding;
mod string;
use string::JavaString;

type Result<T> = std::result::Result<T, StatusCode>;
const BAD: StatusCode = StatusCode::BAD_REQUEST;
const FAILED: StatusCode = StatusCode::INTERNAL_SERVER_ERROR;

pub(super) struct Document {
    schema_version: i32,
    revision: i64,
    model_id: Option<JavaString>,
    bindings: Vec<Option<Binding>>,
}
struct Binding {
    id: Option<JavaString>,
    kind: Option<JavaString>,
    parent_id: Option<JavaString>,
    path: Option<JavaString>,
    aliases: Vec<Option<JavaString>>,
    status: Option<JavaString>,
    derived_from: Vec<Option<JavaString>>,
}

pub(super) fn mutate(request: &[u8], operation: &str) -> Result<Document> {
    let request = binding::request(request)?;
    let expected = request.expected;
    let id = request.id;
    let new_path = request.path;
    let mut document = request.document.ok_or(FAILED)?;
    if document.revision != expected {
        return Err(FAILED);
    }
    let id = id.ok_or(FAILED)?;
    let mut index = None;
    for (position, binding) in document.bindings.iter().enumerate() {
        let binding = binding.as_ref().ok_or(FAILED)?;
        if binding.id.as_ref() == Some(&id) {
            index = Some(position);
            break;
        }
    }
    let index = index.ok_or(FAILED)?;
    let binding = document.bindings[index].as_ref().unwrap();
    if operation == "rename"
        && (!binding
            .status
            .as_ref()
            .is_some_and(|status| status.is("active"))
            || new_path.as_ref().is_none_or(JavaString::is_blank))
    {
        return Err(FAILED);
    }
    if operation != "delete" {
        for (position, candidate) in document.bindings.iter().enumerate() {
            if position == index {
                continue;
            }
            let candidate = candidate.as_ref().ok_or(FAILED)?;
            if !candidate
                .status
                .as_ref()
                .is_some_and(|status| status.is("active"))
            {
                continue;
            }
            let path = if operation == "rename" {
                new_path.as_ref().unwrap()
            } else {
                binding.path.as_ref().ok_or(FAILED)?
            };
            if candidate.path.as_ref() == Some(path) {
                return Err(FAILED);
            }
        }
    }
    let binding = document.bindings[index].as_mut().unwrap();
    let changed = match operation {
        "rename" => {
            let old = binding.path.clone().ok_or(FAILED)?;
            if Some(&old) == new_path.as_ref() {
                false
            } else {
                if !binding.aliases.contains(&Some(old.clone())) {
                    binding.aliases.push(Some(old));
                }
                binding.path = new_path;
                true
            }
        }
        "delete"
            if !binding
                .status
                .as_ref()
                .is_some_and(|status| status.is("tombstone")) =>
        {
            binding.status = Some("tombstone".into());
            true
        }
        "restore"
            if !binding
                .status
                .as_ref()
                .is_some_and(|status| status.is("active")) =>
        {
            binding.status = Some("active".into());
            true
        }
        "delete" | "restore" => false,
        _ => return Err(FAILED),
    };
    if changed {
        document.revision = expected.wrapping_add(1);
    }
    Ok(document)
}

impl Document {
    pub(super) fn bytes(&self) -> Vec<u8> {
        fn string(output: &mut Vec<u8>, value: &Option<JavaString>) {
            if let Some(value) = value {
                value.write(output);
            } else {
                output.extend_from_slice(b"null");
            }
        }
        fn strings(output: &mut Vec<u8>, values: &[Option<JavaString>]) {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                string(output, value);
            }
            output.push(b']');
        }
        let mut output = format!(
            "{{\"schemaVersion\":{},\"revision\":{},\"modelId\":",
            self.schema_version, self.revision
        )
        .into_bytes();
        string(&mut output, &self.model_id);
        output.extend_from_slice(b",\"bindings\":[");
        for (index, binding) in self.bindings.iter().enumerate() {
            if index > 0 {
                output.push(b',');
            }
            let Some(binding) = binding else {
                output.extend_from_slice(b"null");
                continue;
            };
            output.extend_from_slice(b"{\"id\":");
            string(&mut output, &binding.id);
            output.extend_from_slice(b",\"kind\":");
            string(&mut output, &binding.kind);
            output.extend_from_slice(b",\"parentId\":");
            string(&mut output, &binding.parent_id);
            output.extend_from_slice(b",\"path\":");
            string(&mut output, &binding.path);
            output.extend_from_slice(b",\"aliases\":");
            strings(&mut output, &binding.aliases);
            output.extend_from_slice(b",\"status\":");
            string(&mut output, &binding.status);
            output.extend_from_slice(b",\"derivedFrom\":");
            strings(&mut output, &binding.derived_from);
            output.push(b'}');
        }
        output.extend_from_slice(b"]}");
        output
    }
}
