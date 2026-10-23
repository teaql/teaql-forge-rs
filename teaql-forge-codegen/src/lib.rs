mod java_xml;
pub mod xml_factory;
mod auxiliary;
mod business_id;
mod descriptor_context;
mod i18n;
mod index;
mod shadow;
pub mod context;
mod data_service_properties;
pub mod demo_model;
pub mod engine;
pub mod evaluation;
mod file_collation;
pub mod graph_service;
mod hutool_json;
pub mod input;
mod input_path;
mod input_zip;
mod java_doc;
mod java_hash_set;
mod java_stg;
mod java_stg_expr;
mod pluralization;
pub mod preview;
mod preview_beans;
mod preview_domain;
mod preview_domain_collections;
mod preview_fields;
pub mod preview_graph;
mod preview_objects;
mod preview_rust;
mod preview_templates;
pub mod process_properties;
mod rust_engine;
mod rust_assist;
mod rust_bootstrap;
mod rust_checker;
mod rust_entity;
mod rust_expression;
mod rust_globals;
mod rust_request;
mod rust_workspace;
pub mod semantic;
mod sql;
pub mod templates;

#[cfg(test)]
mod descriptor_naming_parity {
    #[test]
    fn all_pinned_evo_pluralization_corpus_strings_match() {
        let evidence: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/expected/descriptor-naming-6f2d0a.json"
        ))
        .unwrap();
        assert_eq!(
            evidence["source_commit"],
            "6f2d0a33699277b943148db948bdec2d5725df37"
        );
        let results = evidence["plural_results"].as_object().unwrap();
        assert_eq!(results.len(), 2224);
        for (word, expected) in results {
            assert_eq!(
                crate::pluralization::pluralize(word),
                expected.as_str().unwrap(),
                "{word:?}"
            );
        }
    }
}
