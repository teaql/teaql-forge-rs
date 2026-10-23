use super::{EvaluationItem, EvaluationResponse};
use roxmltree::Node;
use teaql_forge_model::ir::{Domain, EntityMember};

const JAVA: &str = "abstract assert boolean break byte case catch char class const continue default do double else enum extends final finally float for goto if implements import instanceof int interface long native new null package private protected public return short static strictfp super switch synchronized this throw throws transient try void volatile while true false var yield record sealed non-sealed permits";
const RUST: &str = "as async await become box break const continue crate do dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type typeof unsafe unsized use virtual where while yield try macro union";
const GO: &str = "break default func interface select case defer go map struct chan else goto package switch const fallthrough if range type continue for import return var";
const SWIFT: &str = "associatedtype class deinit enum extension fileprivate func import init inout internal let open operator private precedencegroup protocol public rethrows static struct subscript typealias var break case continue default defer do else fallthrough for guard if in repeat return switch where while as any false is nil self Self super true try";
const DART: &str = "abstract as assert async await break case catch class const continue covariant default deferred do dynamic else enum export extends extension external factory false final finally for Function get hide if implements import in interface is late library mixin new null on operator part required rethrow return set show static super switch sync this throw true try typedef var void when while with yield";
const SQL: &str = include_str!("../../templates/doc/SQL2016.kw");

fn conflicts(name: &str) -> Vec<&'static str> {
    // Java HashMap iteration order for the source rule's six fixed language keys.
    [
        ("Java", JAVA),
        ("Rust", RUST),
        ("Go", GO),
        ("Swift", SWIFT),
        ("Dart", DART),
        ("SQL", SQL),
    ]
    .into_iter()
    .filter_map(|(language, words)| {
        words
            .split_whitespace()
            .any(|word| word == name)
            .then_some(language)
    })
    .collect()
}

fn two_word_name(name: &str, suffix: &str) -> String {
    let normalized = name.replace('-', "_").to_ascii_lowercase();
    let mut candidate = format!("{normalized}_{suffix}");
    let mut discriminator = 2;
    while !conflicts(&candidate).is_empty() {
        candidate = format!("{normalized}_{suffix}{discriminator}");
        discriminator += 1;
    }
    candidate
}

fn source_line(node: Node<'_, '_>) -> usize {
    let text = node.document().input_text();
    let start = node.range().start;
    let offset = text[start..].find('>').unwrap_or(0);
    node.document().text_pos_at(start + offset).row as usize
}

pub(super) fn evaluate(
    root: Node<'_, '_>,
    domain: Option<&Domain>,
    filename: &str,
    response: &mut EvaluationResponse,
) {
    let Some(domain) = domain else { return };
    for entity in &domain.entities {
        let object_name = &entity.name;
        let Some(object) = root
            .children()
            .find(|child| child.is_element() && crate::java_xml::qname(child) == object_name)
        else {
            continue;
        };
        let line = Some(source_line(object));
        let object_conflicts = conflicts(object_name);
        if !object_conflicts.is_empty() {
            let suggestion = two_word_name(object_name, "record");
            response.errors.push(EvaluationItem {
                rule_id: "KSML-KEYWORD-001".into(),
                title: "XML element name conflicts with reserved keyword".into(),
                message: format!("KSML XML naming error: element <{object_name}> uses the reserved name '{object_name}', which conflicts with: {}. Rename the XML element itself to a descriptive two-word snake_case name such as <{suggestion}>, and update references to the old element name.", object_conflicts.join(", ")),
                path: format!("/root/{object_name}"),
                object_name: Some(object_name.clone()),
                field_name: None,
                xml_path: Some(filename.into()),
                line_number: line,
            });
        }
        for member in &entity.members {
            let name = match member {
                EntityMember::Field(field) => field.name.as_str(),
                EntityMember::Relation(relation) => relation.name.as_str(),
            };
            let field_conflicts = conflicts(name);
            if field_conflicts.is_empty() {
                continue;
            }
            let suggestion = match name.to_ascii_lowercase().as_str() {
                "operator" => "operator_id".into(),
                "user" => "user_id".into(),
                _ => two_word_name(object_name, name),
            };
            response.errors.push(EvaluationItem {
                rule_id: "KSML-KEYWORD-002".into(),
                title: "XML attribute name conflicts with reserved keyword".into(),
                message: format!("KSML XML naming error at {filename}:{}: element <{object_name}> defines an XML attribute named '{name}', which conflicts with reserved keywords in: {}. Rename the attribute in the KSML XML itself and keep its current value; for example, change {name}=\"...\" to {suggestion}=\"...\". Also update any <_value> attributes or other model references that use the old field name.", line.unwrap_or(0), field_conflicts.join(", ")),
                path: format!("/root/{object_name}/@{name}"),
                object_name: Some(object_name.clone()),
                field_name: Some(name.into()),
                xml_path: Some(filename.into()),
                line_number: line,
            });
        }
    }
}
