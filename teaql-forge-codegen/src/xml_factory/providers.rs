//! Reflective SAX factory lookup against the pinned source class environment.
use super::{FactoryCause, XmlFactoryError};
use serde::Deserialize;
use std::{collections::{BTreeMap, BTreeSet}, sync::OnceLock};

#[derive(Deserialize)]
struct Factory {
    kind: String,
    public_noarg_constructor: bool,
    constructor_error: Option<XmlFactoryError>,
}

#[derive(Deserialize)]
struct Catalogue {
    classes: BTreeSet<String>,
    factories: BTreeMap<String, Factory>,
    lookup_errors: BTreeMap<String, XmlFactoryError>,
}

fn catalogue() -> &'static Catalogue {
    static DATA: OnceLock<Catalogue> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("../data/jaxp-classes-6f2d0a.json")).expect("pinned JAXP class metadata"))
}

fn failure(provider: &str, cause: &str, detail: Option<String>, missing: bool) -> XmlFactoryError {
    let message = if missing { format!("Provider {provider} not found") }
    else {
        let cause_text = match &detail { Some(message) => format!("{cause}: {message}"), None => cause.to_owned() };
        format!("Provider {provider} could not be instantiated: {cause_text}")
    };
    XmlFactoryError {
        exception_class: "javax.xml.parsers.FactoryConfigurationError".into(),
        message: Some(message.clone()), bad_request: false,
        causes: vec![FactoryCause { class: "javax.xml.parsers.FactoryConfigurationError".into(), message: Some(message) },
            FactoryCause { class: cause.into(), message: detail }],
    }
}

fn array_component_class(provider: &str) -> Option<&str> {
    let component = provider.trim_start_matches('[');
    if component.len() == provider.len() { return None; }
    component.strip_prefix('L').and_then(|name| name.strip_suffix(';'))
}

fn array_exists(provider: &str, data: &Catalogue) -> bool {
    let component = provider.trim_start_matches('[');
    if component.len() == provider.len() { return false; }
    if matches!(component, "B" | "C" | "D" | "F" | "I" | "J" | "S" | "Z") { return true; }
    array_component_class(provider).is_some_and(|name| data.classes.contains(name))
}

fn recursive_msv_failure() -> XmlFactoryError {
    // This provider delegates construction back to SAXParserFactory.newInstance.
    // The source JVM exhausts its stack during recursive initialization. Its
    // first surfaced class-init error depends on the JVM execution mode.
    let mut interpreted = false;
    let mut non_tiered = false;
    for key in ["JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"] {
        if let Ok(value) = std::env::var(key) {
            for option in value.split_whitespace() {
                interpreted |= option == "-Xint";
                non_tiered |= option == "-XX:-TieredCompilation";
            }
        }
    }
    non_tiered &= !interpreted;
    let (class, message) = if non_tiered {
        ("java.lang.BootstrapMethodError", "bootstrap method initialization exception")
    } else {
        ("java.lang.NoClassDefFoundError", "Could not initialize class java.lang.invoke.StringConcatFactory")
    };
    XmlFactoryError {
        exception_class: class.into(),
        message: Some(message.into()),
        bad_request: false,
        causes: vec![FactoryCause { class: class.into(), message: Some(message.into()) }],
    }
}

pub(super) fn validate_sax(provider: &str) -> Result<(), XmlFactoryError> {
    if provider == "com.sun.org.apache.xerces.internal.jaxp.SAXParserFactoryImpl" { return Ok(()); }
    if provider == "com.ctc.wstx.shaded.msv_core.verifier.jaxp.SAXParserFactoryImpl" {
        return Err(recursive_msv_failure());
    }
    let data = catalogue();
    if let Some(error) = data.lookup_errors.get(provider) {
        if error.exception_class != "java.lang.ClassNotFoundException" { return Err(error.clone()); }
    }
    // Class.forName must resolve an object array's component class before
    // JAXP can test whether the array implements SAXParserFactory. That load
    // can fail with a linkage error even though the array name itself is valid.
    if let Some(component) = array_component_class(provider) {
        if let Some(error) = data.lookup_errors.get(component) {
            if error.exception_class != "java.lang.ClassNotFoundException" {
                return Err(error.clone());
            }
        }
    }
    if !data.classes.contains(provider) && !array_exists(provider, data) {
        // The bootstrap-loader fallback reports the binary class name with
        // slash separators, unlike the first application-loader attempt.
        return Err(failure(provider, "java.lang.ClassNotFoundException", Some(provider.replace('.', "/")), true));
    }
    let factory = data.factories.get(provider).filter(|factory| factory.kind == "sax")
        .ok_or_else(|| failure(provider, "java.lang.ClassCastException", Some(format!("{provider} cannot be cast to javax.xml.parsers.SAXParserFactory")), false))?;
    if !factory.public_noarg_constructor {
        let cause = factory.constructor_error.as_ref().expect("source constructor failure");
        return Err(failure(provider, &cause.exception_class, cause.message.clone(), false));
    }
    Ok(())
}
