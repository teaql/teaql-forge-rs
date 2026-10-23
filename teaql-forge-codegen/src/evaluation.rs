//! Shared model evaluation for local generation and HTTP requests.
use serde::Serialize;
use std::fmt::Write;
use teaql_forge_model::ir::Domain;

pub(crate) mod domain;
mod keywords;
mod privacy;
mod rules;

pub struct EvaluationOutcome {
    pub response: EvaluationResponse,
    pub domain: Option<Domain>,
}

pub fn evaluate_model(xml: &str, filename: &str, scope: &str) -> EvaluationOutcome {
    let input = crate::input::ModelInput {
        files: [(filename.into(), xml.as_bytes().to_vec())].into(),
        entrance_file: Some(filename.into()),
        zip_upload: false,
        ..Default::default()
    };
    evaluate_input(&input, scope)
}

pub fn evaluate_input(input: &crate::input::ModelInput, scope: &str) -> EvaluationOutcome {
    try_evaluate_input(input, scope).unwrap_or_else(|error| std::panic::panic_any(error))
}

pub fn try_evaluate_input(input: &crate::input::ModelInput, scope: &str) -> Result<EvaluationOutcome, crate::xml_factory::XmlFactoryError> {
    let resolved = input.resolve();
    if let Some(error) = &resolved.factory_failure {
        // Once the recursive MSV factory has broken JVM class initialization,
        // evaluation of an otherwise readable XML document reaches the catalog
        // message class. Blank inputs fail before that catalog lookup.
        if let Some(catalog_error) = error.catalog_after_recursive_msv() {
            let readable = input.entrance_file.as_deref()
                .and_then(|name| input.files.get(name))
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .is_some_and(|xml| roxmltree::Document::parse_with_options(
                    xml,
                    roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() },
                ).is_ok());
            if readable { return Err(catalog_error); }
        }
        return Err(error.clone());
    }
    let filename = input.entrance_file.as_deref().unwrap_or("model.xml");
    let root_allowed=resolved.documents.first().is_some_and(|source|matches!(source.root_tag.as_str(),"root"|"_root"));
    let raw = if root_allowed && resolved.errors.is_empty() {input.resolve_sax()}else{crate::input::ResolvedModel::default()};
    let raw_failure=raw.entrance_parse_error.as_ref().or_else(||raw.errors.first());
    let domain_error = if root_allowed && resolved.errors.is_empty() {
        if let Some(error)=raw_failure {
            Some(EvaluationItem { rule_id:"KSML-XML-005".into(),title:"Domain parse error".into(),
                message:format!("Failed to parse domain model: {}",error.message),path:filename.into(),
                xml_path:Some(filename.into()),line_number:input.files.get(filename).and_then(|bytes|crate::java_xml::root_line(bytes)),
                ..Default::default() })
        } else { domain::diagnostic(&raw,filename) }
    } else {None};
    let included_objects = resolved
        .documents
        .iter()
        .skip(1)
        .flat_map(|source| {
            let document = roxmltree::Document::parse(&source.xml).expect("parsed document");
            document
                .root_element()
                .children()
                .filter(|node| node.is_element() && !crate::java_xml::qname(node).starts_with('_'))
                .map(|node| crate::java_xml::qname(node).to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    let mut outcome = if resolved.documents.is_empty() {
        EvaluationOutcome {
            response: EvaluationResponse::default(),
            domain: None,
        }
    } else {
        rules::evaluate_model(
            &resolved.xml,
            filename,
            scope,
            (root_allowed && resolved.errors.is_empty() && domain_error.is_none())
                .then(|| crate::java_xml::parse_model(&raw, filename).ok())
                .flatten(),
            input.semantic_document(),
            &included_objects,
        )
    };
    resolved.restore_locations(&mut outcome);
    outcome
        .response
        .solids
        .retain(|item| !item.rule_id.starts_with("KSML-UPLOAD-") && item.rule_id != "KSML-XML-002");
    outcome
        .response
        .warnings
        .retain(|item| item.rule_id != "KSML-UPLOAD-004");
    let mut upload = EvaluationResponse::default();
    let upload_item = |rule_id: &str, title: &str, message: String, path: &str| EvaluationItem {
        rule_id: rule_id.into(),
        title: title.into(),
        message,
        path: path.into(),
        ..Default::default()
    };
    let root_location = |mut item: EvaluationItem| {
        if let Some(source) = resolved
            .documents
            .first()
            .filter(|source| source.root_tag == "root")
        {
            item.xml_path = Some(source.filename.clone());
            item.line_number = Some(source.root_line);
        }
        item
    };
    if input.files.is_empty() {
        upload.errors.push(upload_item(
            "KSML-UPLOAD-001",
            "Missing model upload",
            "The upload contains no files.".into(),
            "model.zip",
        ));
    } else {
        upload.solids.push(upload_item(
            "KSML-UPLOAD-001",
            "Model upload is non-empty",
            format!("The upload contains {} files.", input.files.len()),
            "model.zip",
        ));
        let count = input
            .files
            .keys()
            .filter(|name| name.ends_with("main.xml"))
            .count();
        if input.zip_upload && count == 0 {
            upload.errors.push(upload_item(
                "KSML-UPLOAD-002",
                "Missing zip entrypoint",
                "Zip upload must contain exactly one file named main.xml.".into(),
                "model.zip",
            ));
        } else if input.zip_upload && count > 1 {
            upload.errors.push(upload_item(
                "KSML-UPLOAD-003",
                "Duplicate zip entrypoint",
                "Zip upload contains multiple files named main.xml.".into(),
                "model.zip",
            ));
        } else if input.entrance_file.is_some() {
            if !input.zip_upload && !filename.ends_with(".xml") && !filename.ends_with(".ksml") {
                upload.warnings.push(root_location(upload_item(
                    "KSML-UPLOAD-004",
                    "Non-standard file extension",
                    "Single-file upload should preferably end with .xml or .ksml.".into(),
                    filename,
                )));
            } else {
                upload.solids.push(root_location(upload_item(
                    "KSML-UPLOAD-005",
                    "Valid entrypoint found",
                    format!("Resolved entrypoint: {filename}"),
                    filename,
                )));
            }
        }
    }
    upload.errors.extend(
        resolved
            .errors
            .iter()
            .filter(|item| item.rule_id.starts_with("KSML-UPLOAD-"))
            .cloned(),
    );
    upload
        .errors
        .extend(resolved.entrance_parse_error.iter().cloned());
    upload.errors.extend(
        resolved
            .errors
            .iter()
            .filter(|item| item.rule_id.starts_with("KSML-XML-"))
            .cloned(),
    );
    if let Some(error) = domain_error.clone() {
        upload.errors.push(error);
    }
    for (index, source) in resolved.documents.iter().enumerate() {
        if domain_error.is_some() {
            break;
        }
        upload.solids.push(EvaluationItem {
            rule_id: "KSML-XML-002".into(),
            title: "XML is well-formed".into(),
            message: format!(
                "The XML document {} was parsed successfully.",
                source.filename
            ),
            path: source.filename.clone(),
            xml_path: (source.root_tag == "root").then(|| source.filename.clone()),
            line_number: (source.root_tag == "root").then_some(source.root_line),
            ..Default::default()
        });
        let document = roxmltree::Document::parse(&source.xml).expect("parsed source");
        for wrapper in ["model", "ksml", "domain"] {
            if let Some(node) = document
                .descendants()
                .find(|node| node.is_element() && crate::java_xml::qname(node) == wrapper)
            {
                let path = node
                    .ancestors()
                    .filter(|node| node.is_element())
                    .map(|node| format!("/{}", crate::java_xml::qname(node)))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>();
                upload.errors.push(EvaluationItem { rule_id: "KSML-XML-004".into(), title: "Invalid wrapper tag".into(), message: format!("The tag <{wrapper}> is invalid. Do not wrap model objects inside <{wrapper}>."), path: if index == 0 { path } else { format!("{}:{path}", source.filename) }, xml_path: Some(source.filename.clone()), line_number: Some(document.text_pos_at(node.range().start).row as usize), ..Default::default() });
            }
        }
    }
    upload.errors.append(&mut outcome.response.errors);
    outcome.response.errors = upload.errors;
    upload.warnings.append(&mut outcome.response.warnings);
    outcome.response.warnings = upload.warnings;
    upload.solids.append(&mut outcome.response.solids);
    outcome.response.solids = upload.solids;
    resolved.restore_source_index(&mut outcome.response);
    Ok(outcome)
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationResponse {
    pub errors: Vec<EvaluationItem>,
    pub warnings: Vec<EvaluationItem>,
    pub suggestions: Vec<EvaluationItem>,
    pub solids: Vec<EvaluationItem>,
    pub privacy_findings: Vec<PrivacyFinding>,
}

impl EvaluationResponse {
    pub fn blocks_generation(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationItem {
    pub rule_id: String,
    pub title: String,
    pub message: String,
    pub path: String,
    pub object_name: Option<String>,
    pub field_name: Option<String>,
    pub xml_path: Option<String>,
    pub line_number: Option<usize>,
}

impl EvaluationItem {
    fn action(&self) -> &'static str {
        match self.rule_id.as_str() {
            id if id.starts_with("KSML-KEYWORD-") => {
                "Edit the KSML XML name using the concrete snake_case example."
            }
            "KSML-GRAPH-001" => {
                "Edit the KSML XML and delete one listed reference attribute; do not merely rename the attribute."
            }
            "KSML-GRAPH-002" => {
                "Shorten the reported chain or replace a descriptive reference with a string field."
            }
            "KSML-CONSTANT-008" => {
                "Add the missing field to the reported <_value>; every constant value must explicitly provide every declared constant field."
            }
            _ => "Check agents/ERROR-FIX.md",
        }
    }

    fn error_target(&self) -> String {
        if self.rule_id.starts_with("KSML-KEYWORD-") || self.object_name.is_none() {
            return self.path.clone();
        }
        self.object_field_target()
    }

    fn object_field_target(&self) -> String {
        match (&self.object_name, &self.field_name) {
            (Some(object), Some(field)) => format!("{object}.{field}"),
            (Some(object), None) => object.clone(),
            (None, _) => String::new(),
        }
    }

    fn source(&self) -> String {
        match (&self.xml_path, self.line_number) {
            (Some(path), Some(line)) => format!("`{path}:{line}`"),
            (Some(path), None) => format!("`{path}`"),
            (None, _) => "—".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyFinding {
    pub severity: String,
    pub target: String,
    pub reason: String,
    pub recommended_action: String,
    pub regulatory_basis: String,
}

struct ErrorSummary<'a> {
    rule_id: &'a str,
    category: &'a str,
    message: &'a str,
    action: &'a str,
    count: usize,
}

fn error_summaries(errors: &[EvaluationItem]) -> Vec<ErrorSummary<'_>> {
    let mut summaries: Vec<ErrorSummary<'_>> = Vec::new();
    for error in errors {
        if let Some(summary) = summaries
            .iter_mut()
            .find(|summary| summary.rule_id == error.rule_id)
        {
            summary.count += 1;
            continue;
        }
        let (category, message, action) = match error.rule_id.as_str() {
            "KSML-STRUCTURE-004" => (
                "field-as-child-element",
                "Business object fields were written as XML child elements. Did you mean to use XML attributes?",
                "Move each child value onto the parent object, for example <school name=\"...\" phone=\"...\"/>. Only constant object values should be <_value> children.",
            ),
            "KSML-GRAPH-001" => (
                "recursive-reference-chain",
                error.message.as_str(),
                "Edit the KSML XML and delete one of the exact reference attributes listed in the error. Do not merely rename the attribute.",
            ),
            "KSML-GRAPH-002" => (
                "reference-depth-exceeded",
                error.message.as_str(),
                "Edit the KSML XML and remove one listed nested reference attribute, or change a descriptive entity reference to string().",
            ),
            "KSML-PRIVACY-001-PROHIBITED" => (
                "prohibited-authentication-data",
                error.message.as_str(),
                "Delete the exact field from the KSML model. Masking or encryption does not make persistent storage acceptable.",
            ),
            "KSML-PRIVACY-001-PASSWORD" => (
                "plaintext-password-field",
                error.message.as_str(),
                "Rename password to password_hash, store only a non-reversible hash, and add the hash field to _audit_mask_fields.",
            ),
            id if id.starts_with("KSML-KEYWORD-") => (
                "reserved-keyword",
                error.message.as_str(),
                "Rename the target to a descriptive two-word snake_case name using the concrete example in the error.",
            ),
            _ => (
                error.title.as_str(),
                error.message.as_str(),
                "Check agents/ERROR-FIX.md, apply the smallest fix, then rerun evaluation.",
            ),
        };
        summaries.push(ErrorSummary {
            rule_id: &error.rule_id,
            category,
            message,
            action,
            count: 1,
        });
    }
    summaries
}

/// Render the source generator's `generator/evaluation/eval-report.md.stg` contract.
pub fn render_markdown_report(response: &EvaluationResponse) -> String {
    let mut out = format!(
        "<!-- ephemeral -->\n# KSML Evaluation Report\n## 📊 Summary\n- **Errors**: {}\n- **Warnings**: {}\n- **Suggestions**: {}\n- **Solids**: {}\n\n",
        response.errors.len(),
        response.warnings.len(),
        response.suggestions.len(),
        response.solids.len()
    );
    if !response.errors.is_empty() {
        let _ = write!(
            out,
            "## Error Pattern Summary\n\n{} errors total. Fix the largest repeated pattern first, then rerun evaluation.\n\n| Count | Pattern | Explanation | Action |\n|-------|---------|-------------|--------|\n",
            response.errors.len()
        );
        for summary in error_summaries(&response.errors) {
            let _ = writeln!(
                out,
                "| {} | `{}` (`{}`) | {} | {} |",
                summary.count, summary.category, summary.rule_id, summary.message, summary.action
            );
        }
        out.push_str("\n\n## ❌ Errors (Must Fix)\n\n| Rule ID | Target | Source | Error Message | Action |\n|---------|--------|--------|---------------|--------|\n");
        for error in &response.errors {
            let _ = writeln!(
                out,
                "| `{}` | `{}` | {} | {} | {} |",
                error.rule_id,
                error.error_target(),
                error.source(),
                error.message,
                error.action()
            );
        }
        out.push_str("\n\n");
    }
    if !response.warnings.is_empty() {
        out.push_str("## ⚠️ Warnings\n\n| Rule ID | Target | Source | Warning Message |\n|---------|--------|--------|-----------------|\n");
        for item in &response.warnings {
            let _ = writeln!(
                out,
                "| `{}` | `{}` | {} | {} |",
                item.rule_id,
                item.object_field_target(),
                item.source(),
                item.message
            );
        }
        out.push_str("\n\n");
    }
    if !response.suggestions.is_empty() {
        out.push_str("## 💡 Suggestions\n\n| Rule ID | Target | Source | Suggestion |\n|---------|--------|--------|------------|\n");
        for item in &response.suggestions {
            let _ = writeln!(
                out,
                "| `{}` | `{}` | {} | {} |",
                item.rule_id,
                item.object_field_target(),
                item.source(),
                item.message
            );
        }
        out.push_str("\n\n");
    }
    if !response.privacy_findings.is_empty() {
        out.push_str("## Privacy Findings\n\n| Severity | Target | Reason | Recommended Action | Regulatory Basis |\n|----------|--------|--------|--------------------|------------------|\n");
        for finding in &response.privacy_findings {
            let _ = writeln!(
                out,
                "| {} | `{}` | {} | {} | {} |",
                finding.severity,
                finding.target,
                finding.reason,
                finding.recommended_action,
                finding.regulatory_basis
            );
        }
        out.push_str("\n\n");
    }
    if !response.solids.is_empty() {
        out.push_str("## ✅ Solids\n\n| Rule ID | Target | Source | Message |\n|---------|--------|--------|---------|\n");
        for item in &response.solids {
            let _ = writeln!(
                out,
                "| `{}` | `{}` | {} | {} |",
                item.rule_id,
                item.object_field_target(),
                item.source(),
                item.message
            );
        }
        out.push('\n');
    }
    // ST4's AutoIndentWriter discards a standalone carriage return while
    // rendering an interpolated value, and writes CRLF as LF. Model attribute
    // references can contain both via XML character references.
    out.replace("\r\n", "\n").replace('\r', "")
}
