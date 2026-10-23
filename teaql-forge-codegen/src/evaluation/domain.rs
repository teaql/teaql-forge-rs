use super::EvaluationItem;
use crate::input::ResolvedModel;
use roxmltree::Document;
use teaql_forge_model::{
    ir::{Domain, EntityMember},
    parser::parse_model,
};

pub(crate) fn empty_attribute_diagnostic(sources: &ResolvedModel, _filename: &str) -> Option<EvaluationItem> {
    let document = Document::parse_with_options(&sources.model_xml, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() }).ok()?;
    let root = document.root_element();
    if !matches!(crate::java_xml::qname(root), "root" | "_root") {
        return None;
    }
    for object in root
        .children()
        .filter(|node| node.is_element() && !crate::java_xml::qname(node).starts_with('_'))
    {
        let text = document.input_text();
        let end = object.range().start + text[object.range().start..].find('>').unwrap_or(0);
        let position = if crate::xml_factory::sax_provider().ok() == Some(crate::xml_factory::SaxProvider::Woodstox) { object.range().start } else { end };
        let line = document.text_pos_at(position).row as usize;
        for attribute in object
            .attributes()
            .filter(|attribute| !attribute.name().starts_with('_'))
        {
            if !teaql_forge_model::field_attributes::empty_value(attribute.value()) {
                continue;
            }
            let name = crate::java_xml::qname(object);
            let field = attribute.name();
            let (source, line) = if crate::xml_factory::sax_provider().ok() == Some(crate::xml_factory::SaxProvider::Woodstox) {
                sources.model_node_source_location(object.range().start)?
            } else { sources.model_source_location(line)? };
            let path = &source.filename;
            return Some(EvaluationItem { rule_id: "KSML-XML-005".into(), title: "Domain parse error".into(), message: format!("Failed to parse domain model: Attribute '{field}' of object '{name}' is empty at line {line} in {path}. Empty values are not allowed."), path: format!("/{name}/@{field}"), object_name: Some(name.into()), field_name: Some(field.into()), xml_path: Some(path.clone()), line_number: Some(line), });
        }
    }
    None
}

pub(super) fn diagnostic(sources: &ResolvedModel, filename: &str) -> Option<EvaluationItem> {
    let document = Document::parse(&sources.model_xml).ok()?;
    let root = document.root_element();
    if !matches!(crate::java_xml::qname(root), "root" | "_root") {
        return None;
    }
    if let Some(error) = empty_attribute_diagnostic(sources, filename) { return Some(error); }
    if root
        .attribute("name")
        .is_none_or(|value| value.trim().is_empty())
    {
        return Some(EvaluationItem { rule_id: "KSML-XML-005".into(), title: "Domain parse error".into(), message: format!("Failed to parse domain model: model root in entrance file({filename}) must define a non-empty name attribute"), path: filename.into(), xml_path: (crate::java_xml::qname(root) == "root").then(|| filename.into()), line_number: (crate::java_xml::qname(root) == "root").then_some(document.text_pos_at(root.range().start).row as usize), ..Default::default() });
    }
    let domain = match parse_model(&sources.model_xml, filename) {
        Ok(domain) => domain,
        Err(teaql_forge_model::error::ParseError::UndefinedDataService(message)
            | teaql_forge_model::error::ParseError::UndefinedTypeParent(message)) => {
            return Some(EvaluationItem {
                rule_id: "KSML-XML-005".into(),
                title: "Domain parse error".into(),
                message: format!("Failed to parse domain model: {message}"),
                path: filename.into(),
                xml_path: (crate::java_xml::qname(root) == "root").then(|| filename.into()),
                line_number: (crate::java_xml::qname(root) == "root").then(|| {
                    sources
                        .documents
                        .first()
                        .map(|source| source.root_line)
                        .unwrap_or(1)
                }),
                ..Default::default()
            });
        }
        Err(_) => return None,
    };
    let mut reverse: Vec<Vec<(usize, String)>> = vec![Vec::new(); domain.entities.len()];
    for (child, entity) in domain.entities.iter().enumerate() {
        for member in &entity.members {
            let EntityMember::Relation(relation) = member else {
                continue;
            };
            if let Some(parent) = domain
                .entities
                .iter()
                .position(|entity| entity.name == relation.target)
            {
                if child != parent {
                    reverse[parent]
                        .push((child, format!("/root/{}/@{}", entity.name, relation.name)));
                }
            }
        }
    }
    for index in 0..domain.entities.len() {
        if let Some(mut error) = explore(index, &domain, &reverse, &[], &[]) {
            let parts = error.path.split('/').collect::<Vec<_>>();
            if let Some(object) = sources
                .documents
                .first()
                .and_then(|source| Document::parse(&source.xml).ok())
                .and_then(|doc| {
                    let object = doc.root_element().children().find(|node| {
                        node.is_element() && Some(crate::java_xml::qname(node)) == parts.get(2).copied()
                    })?;
                    let field = parts.get(3)?.strip_prefix('@')?;
                    Some((
                        filename.to_owned(),
                        doc.text_pos_at(object.attribute_node(field)?.range().start)
                            .row as usize,
                    ))
                })
            {
                error.xml_path = Some(object.0);
                error.line_number = Some(object.1);
            }
            return Some(error);
        }
    }
    None
}

fn explore(
    index: usize,
    domain: &Domain,
    reverse: &[Vec<(usize, String)>],
    path: &[usize],
    references: &[String],
) -> Option<EvaluationItem> {
    let cycle = path.iter().position(|candidate| *candidate == index);
    if cycle.is_some() || path.len() > 20 {
        let start = cycle.unwrap_or(0);
        let chain = path[start..]
            .iter()
            .copied()
            .chain(std::iter::once(index))
            .map(|index| domain.entities[index].name.as_str())
            .collect::<Vec<_>>()
            .join(" -> ");
        let refs = &references[start..];
        let joined = refs.join(", ");
        let message = if cycle.is_some() {
            format!("KSML XML reference cycle detected. Element cycle: {chain}. The evaluator followed these XML reference attributes: {joined}. Edit the KSML XML itself and delete at least one of those reference attributes to break the cycle. Do not merely rename the attribute. If a relationship only stores descriptive data, change its value from an entity reference such as target() to string().")
        } else {
            format!("The KSML XML reference graph exceeds the maximum supported depth of 20. Deepest explored element chain: {chain}. XML reference attributes on this chain: {joined}. Edit the KSML XML itself and delete at least one nested reference attribute, or change a descriptive entity reference to string().")
        };
        return Some(EvaluationItem {
            rule_id: if cycle.is_some() {
                "KSML-GRAPH-001"
            } else {
                "KSML-GRAPH-002"
            }
            .into(),
            title: if cycle.is_some() {
                "Recursive KSML XML reference cycle"
            } else {
                "KSML XML reference depth exceeded"
            }
            .into(),
            message,
            path: refs.first().cloned().unwrap_or_else(|| "/root".into()),
            ..Default::default()
        });
    }
    let mut path = path.to_vec();
    path.push(index);
    for (child, reference) in &reverse[index] {
        let mut references = references.to_vec();
        references.push(reference.clone());
        if let Some(error) = explore(*child, domain, reverse, &path, &references) {
            return Some(error);
        }
    }
    None
}
