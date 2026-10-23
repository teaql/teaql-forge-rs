//! Model upload loading and recursive include resolution.
use crate::evaluation::{EvaluationItem, EvaluationOutcome};
use crate::semantic::{self, SemanticIdentityDocument};
use roxmltree::Document;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone,Copy,PartialEq)]
enum Reading { Dom, Sax }

fn document(xml: &str) -> Result<Document<'_>, roxmltree::Error> {
    Document::parse_with_options(xml, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() })
}

#[derive(Clone, Debug, Default)]
pub struct ModelInput {
    pub files: BTreeMap<String, Vec<u8>>,
    pub entrance_file: Option<String>,
    pub zip_upload: bool,
    pub source_file_count: Option<u32>,
    pub source_bytes: Option<u64>,
}

pub fn normalize_path(path: &str) -> String {
    let path = crate::input_path::normalize(path);
    let path = path.strip_prefix('/').unwrap_or(&path);
    path.strip_suffix('/').unwrap_or(path).into()
}

impl ModelInput {
    pub fn load(bytes: &[u8], filename: &str) -> Result<Self, String> {
        let mut input = Self::default();
        input.load_into(bytes, filename)?;
        Ok(input)
    }

    pub fn load_into(&mut self, bytes: &[u8], filename: &str) -> Result<(), String> {
        self.source_bytes = Some(bytes.len() as u64);
        self.zip_upload = crate::input_zip::is_zip(bytes, filename);
        if self.zip_upload {
            let mut count = 0;
            crate::input_zip::read(bytes, |name, content| {
                if name.ends_with('/')
                    || name.contains("__MACOSX")
                    || ((name.starts_with('.') || name.contains("/."))
                        && name != semantic::SIDECAR_PATH)
                {
                    return;
                }
                let name = normalize_path(&name);
                if self.entrance_file.is_none() && name.ends_with("main.xml") {
                    self.entrance_file = Some(name.clone());
                }
                self.files.insert(name, content);
                count += 1;
            })?;
            self.source_file_count = Some(count);
        } else {
            self.entrance_file = Some(filename.into());
            self.files.insert(normalize_path(filename), bytes.to_vec());
            self.source_file_count = Some(1);
        }
        Ok(())
    }

    pub fn semantic_document(&self) -> Result<Option<SemanticIdentityDocument>, String> {
        self.files
            .get(semantic::SIDECAR_PATH)
            .map(|bytes| semantic::load_document(bytes))
            .transpose()
    }

    pub fn evaluate(&self, scope: &str) -> EvaluationOutcome {
        crate::evaluation::evaluate_input(self, scope)
    }

    pub fn try_evaluate(&self, scope: &str) -> Result<EvaluationOutcome, crate::xml_factory::XmlFactoryError> {
        crate::evaluation::try_evaluate_input(self, scope)
    }

    pub fn preview_factory_failure(&self) -> Option<crate::xml_factory::XmlFactoryError> {
        let resolved = self.resolve_sax();
        if let Some(error) = resolved.factory_failure { return Some(error); }
        if resolved.entrance_parse_error.is_some() || !resolved.errors.is_empty() { return None; }
        let parsed = document(&resolved.model_xml).ok()?;
        if parsed.root_element().attribute("name").is_none_or(|value| value.chars().all(crate::input_path::blank_char)) {
            return Some(crate::xml_factory::XmlFactoryError::illegal_argument(format!(
                "model root in entrance file({}) must define a non-empty name attribute", self.entrance_file.as_deref()?
            )));
        }
        None
    }

    pub fn parse_domain(&self) -> Result<teaql_forge_model::ir::Domain, String> {
        if let Some(filename) = self.entrance_file.as_deref() {
            if !self.files.contains_key(filename) {
                return Err(format!(
                    "entrance file({filename}) not found, available files: [{}]",
                    self.files.keys().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
        }
        let resolved = self.resolve_sax();
        if let Some(error) = &resolved.factory_failure {
            return Err(error.to_string());
        }
        if let Some(error) = resolved
            .entrance_parse_error
            .as_ref()
            .or(resolved.errors.first())
        {
            return Err(error.message.clone());
        }
        let filename = self
            .entrance_file
            .as_deref()
            .ok_or("missing entrance file: main.xml")?;
        let parsed = document(&resolved.model_xml).map_err(|error| error.to_string())?;
        if parsed.root_element().attribute("name").is_none_or(|value| value.chars().all(crate::input_path::blank_char)) {
            return Err(format!("model root in entrance file({filename}) must define a non-empty name attribute"));
        }
        if let Some(error) = crate::evaluation::domain::empty_attribute_diagnostic(&resolved, filename) {
            return Err(error.message.strip_prefix("Failed to parse domain model: ").unwrap_or(&error.message).to_owned());
        }
        let domain = crate::java_xml::parse_model(&resolved, filename)
            .map_err(|error| error.to_string())?;
        let mut outcome = EvaluationOutcome {
            response: Default::default(),
            domain: Some(domain),
        };
        resolved.restore_locations(&mut outcome);
        outcome.domain.ok_or_else(|| "missing parsed domain".into())
    }

    /// Direct preview parsing reports the XML parser's exception. Evaluation
    /// instead turns the same failure into a KSML finding with file location.
    pub fn parse_preview_domain(&self) -> Result<teaql_forge_model::ir::Domain, String> {
        self.parse_domain()
    }

    pub fn fatal_parse_failure(&self) -> Option<String> {
        let source = self.resolve();
        if source.entrance_parse_error.is_some() || !source.errors.is_empty()
            || source.documents.first().is_none_or(|doc| !matches!(doc.root_tag.as_str(),"root"|"_root")) {return None;}
        let raw=self.resolve_sax();
        if raw.entrance_parse_error.is_some() || !raw.errors.is_empty(){return None;}
        let parsed=document(&raw.model_xml).ok()?;
        teaql_forge_model::types::has_inheritance_cycle(&parsed)
            .then(|| teaql_forge_model::error::ParseError::TypeInheritanceCycle.to_string())
    }

    pub(crate) fn resolve(&self) -> ResolvedModel { self.resolve_reading(Reading::Dom) }

    pub(crate) fn resolve_sax(&self) -> ResolvedModel { self.resolve_reading(Reading::Sax) }

    fn resolve_reading(&self, reading: Reading) -> ResolvedModel {
        let mut resolved = ResolvedModel::default();
        if let Some(entrance) = &self.entrance_file {
            resolved.visit(self, entrance, None, &mut HashSet::new(), reading);
        }
        if resolved.factory_failure.is_none() {
            resolved.compose();
        }
        resolved
    }

}

#[derive(Default)]
pub(crate) struct ResolvedModel {
    pub xml: String,
    pub model_xml: String,
    pub documents: Vec<SourceDocument>,
    pub errors: Vec<EvaluationItem>,
    pub entrance_parse_error: Option<EvaluationItem>,
    pub factory_failure: Option<crate::xml_factory::XmlFactoryError>,
    segments: Vec<SourceSegment>,
    model_segments: Vec<SourceSegment>,
}

pub(crate) struct SourceDocument {
    pub filename: String,
    pub xml: String,
    pub root_tag: String,
    pub root_line: usize,
    pub source_index: Vec<crate::java_xml::Location>,
    sax_node_lines: BTreeMap<usize, usize>,
}

struct SourceSegment {
    start_offset: usize,
    end_offset: usize,
    source_offset: usize,
    start_line: usize,
    end_line: usize,
    source_line: usize,
    document: usize,
}

impl ResolvedModel {
    fn visit(
        &mut self,
        input: &ModelInput,
        filename: &str,
        origin: Option<EvaluationItem>,
        active: &mut HashSet<String>,
        reading: Reading,
    ) {
        if self.factory_failure.is_some() { return; }
        if active.contains(filename) {
            let mut error = origin.unwrap_or_default();
            error.rule_id = "KSML-UPLOAD-007".into();
            error.title = "Recursive include".into();
            error.message = format!("Include cycle reaches '{filename}'.");
            self.errors.push(error);
            return;
        }
        if self
            .documents
            .iter()
            .any(|document| document.filename == filename)
        {
            return;
        }
        let Some(bytes) = input.files.get(filename) else {
            if let Some(mut error) = origin {
                error.rule_id = "KSML-UPLOAD-006".into();
                error.title = "Missing include file".into();
                error.message = format!("Included file '{filename}' is missing in the upload.");
                self.errors.push(error);
            }
            return;
        };
        let (xml, sax_node_lines) = match if reading == Reading::Dom { crate::java_xml::dom(bytes).map(|xml| (xml, BTreeMap::new())) } else { crate::java_xml::sax_document(bytes) } {
            Ok(document) => document,
            Err(failure) => {
                if let Some(error) = failure.factory {
                    self.factory_failure = Some(error);
                    return;
                }
                let error = EvaluationItem {
                    rule_id: "KSML-XML-001".into(), title: "XML parse error".into(),
                    message: if reading == Reading::Dom { format!("The XML in {filename} is malformed: {}",failure.message) } else { failure.message },
                    path: filename.into(), xml_path: Some(filename.into()), line_number: failure.line,
                    ..Default::default()
                };
                if origin.is_none() { self.entrance_parse_error = Some(error); }
                else { self.errors.push(error); }
                return;
            }
        };
        let document = document(&xml).expect("validated XML");
        let root = document.root_element();
        let root_tag = if reading == Reading::Dom {crate::java_xml::qname(root)}else{root.tag_name().name()}.to_string();
        let root_line = document.text_pos_at(root.range().start).row as usize;
        let is_entrance = self.documents.is_empty();
        let includes = root
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "_include")
            .filter_map(|node| {
                let relative = node
                    .attribute("file")
                    .filter(|value| !value.trim().is_empty())?;
                let path = node
                    .ancestors()
                    .filter(|node| node.is_element())
                    .map(|node| format!("/{}", node.tag_name().name()))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>();
                Some((
                    {
                        let path =
                            crate::input_path::normalize(&format!("{filename}/../{relative}"));
                        path.strip_prefix('/').unwrap_or(&path).to_string()
                    },
                    EvaluationItem {
                        path: if is_entrance {
                            path
                        } else {
                            format!("{filename}:{path}")
                        },
                        xml_path: Some(filename.into()),
                        line_number: Some(document.text_pos_at(node.range().start).row as usize),
                        ..Default::default()
                    },
                ))
            })
            .collect::<Vec<_>>();
        drop(document);
        self.documents.push(SourceDocument {
            sax_node_lines,
            filename: filename.into(),
            xml,
            root_tag,
            root_line,
            source_index:crate::java_xml::source_locations(bytes),
        });
        active.insert(filename.into());
        for (included, origin) in includes {
            self.visit(input, &included, Some(origin), active, reading);
        }
        active.remove(filename);
    }

    fn compose(&mut self) {
        let Some(entrance) = self.documents.first() else {
            return;
        };
        if self.documents.len() == 1 {
            self.xml = entrance.xml.clone();
            self.model_xml = self.xml.clone();
            return;
        }
        let entrance_document = document(&entrance.xml).expect("parsed source");
        let root = entrance_document.root_element();
        let end = root.range().start + entrance.xml[root.range().start..].find('>').unwrap() + 1;
        self.xml = entrance.xml[..end].trim_end_matches('/').to_string();
        if self.xml.ends_with("/>") {
            self.xml.truncate(self.xml.len() - 2);
            self.xml.push('>');
        }
        self.xml.push('\n');
        for (index, source) in self.documents.iter().enumerate() {
            let document = document(&source.xml).expect("parsed source");
            for child in document
                .root_element()
                .children()
                .filter(|node| node.is_element())
            {
                let start_line = self.xml.bytes().filter(|byte| *byte == b'\n').count() + 1;
                let text = &source.xml[child.range()];
                let start_offset = self.xml.len();
                self.xml.push_str(text);
                let end_line = start_line + text.bytes().filter(|byte| *byte == b'\n').count();
                self.segments.push(SourceSegment {
                    start_offset,
                    end_offset: self.xml.len(),
                    source_offset: child.range().start,
                    start_line,
                    end_line,
                    source_line: document.text_pos_at(child.range().start).row as usize,
                    document: index,
                });
                self.xml.push('\n');
            }
        }
        self.xml.push_str(&format!("</{}>", entrance.root_tag));
        let root_tag = entrance.root_tag.clone();
        let mut chunks = Vec::new();
        let mut attributes = Vec::new();
        self.model_chunks(0, &mut HashSet::new(), &mut chunks, &mut attributes);
        self.model_xml = format!("<{root_tag}");
        for (name, value) in attributes {
            let escaped = value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('"', "&quot;")
                .replace('\n', "&#10;")
                .replace('\r', "&#13;")
                .replace('\t', "&#9;");
            self.model_xml.push_str(&format!(" {name}=\"{escaped}\""));
        }
        self.model_xml.push_str(">\n");
        for (index, text, source_line, source_offset) in chunks {
            let start_line = self.model_xml.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let start_offset = self.model_xml.len();
            self.model_xml.push_str(&text);
            let end_line = start_line + text.bytes().filter(|byte| *byte == b'\n').count();
            self.model_segments.push(SourceSegment {
                start_offset,
                end_offset: self.model_xml.len(),
                source_offset,
                start_line,
                end_line,
                source_line,
                document: index,
            });
            self.model_xml.push('\n');
        }
        self.model_xml.push_str(&format!("</{root_tag}>"));
    }

    fn model_chunks(
        &self,
        index: usize,
        active: &mut HashSet<usize>,
        chunks: &mut Vec<(usize, String, usize, usize)>,
        attributes: &mut Vec<(String, String)>,
    ) {
        if !active.insert(index) {
            return;
        }
        let source = &self.documents[index];
        let document = document(&source.xml).expect("parsed source");
        for attribute in document.root_element().attributes() {
            teaql_forge_model::attributes::merge_xml_attribute(
                attributes,
                attribute.name(),
                attribute.value(),
            );
        }
        for child in document
            .root_element()
            .children()
            .filter(|node| node.is_element())
        {
            if child.tag_name().name() == "_include" {
                if let Some(file) = child.attribute("file") {
                    let filename =
                        crate::input_path::normalize(&format!("{}/../{file}", source.filename));
                    if let Some(included) = self
                        .documents
                        .iter()
                        .position(|document| document.filename == filename)
                    {
                        self.model_chunks(included, active, chunks, attributes);
                        continue;
                    }
                }
            }
            chunks.push((
                index,
                source.xml[child.range()].into(),
                document.text_pos_at(child.range().start).row as usize,
                child.range().start,
            ));
        }
        active.remove(&index);
    }

    pub fn model_source_location(&self, line: usize) -> Option<(&SourceDocument, usize)> {
        self.model_segments
            .iter()
            .find(|segment| (segment.start_line..=segment.end_line).contains(&line))
            .map(|segment| {
                (
                    &self.documents[segment.document],
                    segment.source_line + line - segment.start_line,
                )
            })
            .or_else(|| self.source_location(line))
    }

    pub(crate) fn model_node_source_location(&self, offset: usize) -> Option<(&SourceDocument, usize)> {
        let (source, position) = if self.model_segments.is_empty() {
            (self.documents.first()?, offset)
        } else {
            let segment = self.model_segments.iter().find(|s| (s.start_offset..s.end_offset).contains(&offset))?;
            (&self.documents[segment.document], segment.source_offset + offset - segment.start_offset)
        };
        let line = source.sax_node_lines.get(&position).copied().or_else(|| {
            document(&source.xml).ok().map(|doc| doc.text_pos_at(position).row as usize)
        })?;
        Some((source, line))
    }

    pub fn source_location(&self, line: usize) -> Option<(&SourceDocument, usize)> {
        if self.segments.is_empty() {
            return self.documents.first().map(|document| (document, line));
        }
        self.segments
            .iter()
            .find(|segment| (segment.start_line..=segment.end_line).contains(&line))
            .map(|segment| {
                (
                    &self.documents[segment.document],
                    segment.source_line + line - segment.start_line,
                )
            })
            .or_else(|| self.documents.first().map(|document| (document, line)))
    }

    pub(crate) fn restore_source_index(&self, response:&mut crate::evaluation::EvaluationResponse) {
        for item in response.errors.iter_mut().chain(&mut response.warnings).chain(&mut response.suggestions).chain(&mut response.solids) {
            if item.rule_id=="KSML-XML-001" || (item.rule_id=="KSML-XML-005" && item.field_name.is_some())
                || item.rule_id.starts_with("KSML-KEYWORD-") || item.rule_id.starts_with("KSML-PRIVACY-") || item.rule_id.starts_with("KSML-BUSINESS-ID-") || item.rule_id.starts_with("KSML-NAMING-BOOLEAN") {continue;}
            let (file,path)=if let Some((file,path))=item.path.split_once(":/"){(Some(file),format!("/{path}"))}
                else if self.documents.iter().any(|doc|doc.filename==item.path){(Some(item.path.as_str()),"/root".into())}
                else if item.path.starts_with('/') {(item.xml_path.as_deref().or_else(||self.documents.first().map(|doc|doc.filename.as_str())),item.path.clone())}
                else {continue};
            let Some(source)=self.documents.iter().find(|doc|Some(doc.filename.as_str())==file)else{continue};
            let Ok(doc)=document(&source.xml)else{continue};let mut node=doc.root_element();let segments=path.split('/').collect::<Vec<_>>();
            if segments.get(1).copied()!=Some(crate::java_xml::qname(node)){continue;}
            let mut attribute=None;
            for segment in segments.iter().skip(2) {
                if let Some(name)=segment.strip_prefix('@'){attribute=Some(name);break;}
                let (name,position)=segment.split_once('[').map(|(name,n)|(name,n.trim_end_matches(']').parse::<usize>().unwrap_or(1))).unwrap_or((segment,1));
                if let Some(child)=node.children().filter(|child|child.is_element() && crate::java_xml::qname(*child)==name).nth(position.saturating_sub(1)){node=child;}
                else {attribute=Some(segment);break;}
            }
            let index=doc.descendants().filter(|node|node.is_element()).position(|element|element==node).unwrap();
            let location=source.source_index.get(index);
            item.xml_path=Some(source.filename.clone());
            item.line_number=attribute.and_then(|name|location.and_then(|location|location.attributes.get(name).copied())).or_else(||location.and_then(|location|location.line));
        }
    }

    pub fn restore_locations(&self, outcome: &mut EvaluationOutcome) {
        for item in outcome
            .response
            .errors
            .iter_mut()
            .chain(&mut outcome.response.warnings)
            .chain(&mut outcome.response.suggestions)
            .chain(&mut outcome.response.solids)
        {
            if item.xml_path.as_deref()
                != self
                    .documents
                    .first()
                    .map(|document| document.filename.as_str())
            {
                continue;
            }
            if let Some((source, source_line)) =
                item.line_number.and_then(|line| self.source_location(line))
            {
                item.xml_path = Some(source.filename.clone());
                item.line_number = Some(source_line);
                // These source rules attach location through an unindexed object
                // path. The source index resolves its first occurrence in that
                // file, including when the finding concerns a later definition.
                if matches!(
                    item.rule_id.as_str(),
                    "KSML-STRUCTURE-002" | "KSML-BUSINESS-001" | "KSML-OBJECT-001"
                ) {
                    if let Ok(document) = document(&source.xml) {
                        let name = item.path.rsplit('/').next().unwrap_or("");
                        if let Some(object) = document
                            .root_element()
                            .children()
                            .find(|node| node.is_element() && node.tag_name().name() == name)
                        {
                            item.line_number =
                                Some(document.text_pos_at(object.range().start).row as usize);
                        }
                    }
                }
                if source.filename != self.documents[0].filename && item.path.starts_with("/root") {
                    item.path = format!(
                        "{}:{}",
                        source.filename,
                        item.path
                            .replacen("/root", &format!("/{}", source.root_tag), 1)
                    );
                } else if source.root_tag != "root" && item.path.starts_with("/root") {
                    item.path = item.path.replacen("/root", &format!("/{}",source.root_tag), 1);
                }
            }
        }
        if let Some(domain) = outcome.domain.as_mut().filter(|_| crate::xml_factory::sax_provider().ok() != Some(crate::xml_factory::SaxProvider::Woodstox)) {
            for entity in &mut domain.entities {
                if let Some((source, source_line)) = (entity.xml_path != "null")
                    .then(|| self.model_source_location(entity.line_number))
                    .flatten()
                {
                    entity.xml_path = source.filename.clone();
                    entity.line_number = source_line;
                }
                for member in &mut entity.members {
                    let (line, path) = match member {
                        teaql_forge_model::ir::EntityMember::Field(field) => {
                            (&mut field.line_number, &mut field.xml_path)
                        }
                        teaql_forge_model::ir::EntityMember::Relation(relation) => {
                            (&mut relation.line_number, &mut relation.xml_path)
                        }
                    };
                    if *line != 0 && path != "null" {
                        if let Some((source, source_line)) = self.model_source_location(*line) {
                            *path = source.filename.clone();
                            *line = source_line;
                        }
                    }
                }
            }
        }
    }
}
