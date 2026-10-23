use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum ParseError {
    #[error("Handler dispatch failed; nested exception is java.lang.StackOverflowError")]
    TypeInheritanceCycle,
    #[error("{0}")]
    UndefinedTypeParent(String),
    #[error("{0}")]
    UndefinedDataService(String),
    #[error("Failed to read file")]
    #[diagnostic(code(teaql::io_error))]
    IoError(#[from] std::io::Error),

    #[error("Invalid XML format")]
    #[diagnostic(code(teaql::xml_error))]
    XmlError(#[from] roxmltree::Error),

    #[error("Missing attribute '{attr}' in '{node}'")]
    #[diagnostic(code(teaql::missing_attribute))]
    MissingAttribute {
        node: String,
        attr: String,
        #[source_code]
        src: String,
        #[label("here")]
        span: (usize, usize),
    },

    #[error("Invalid attribute '{attr}' in '{node}': {value}")]
    #[diagnostic(code(teaql::invalid_attribute))]
    InvalidAttribute {
        node: String,
        attr: String,
        value: String,
        #[source_code]
        src: String,
        #[label("invalid value here")]
        span: (usize, usize),
    },

    #[error("Domain tag is missing")]
    #[diagnostic(code(teaql::missing_domain))]
    MissingDomain,
}
