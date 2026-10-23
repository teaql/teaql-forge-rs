//! Configuration errors raised before a JAXP parser can read an XML stream.
use serde::{Deserialize, Serialize};
mod providers;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ParserFactory {
    Dom,
    Sax,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaxProvider {
    Jdk,
    Woodstox,
}

pub(crate) fn sax_provider() -> Result<SaxProvider, XmlFactoryError> {
    let config = factory_configuration(ParserFactory::Sax)?;
    let provider = crate::process_properties::property("javax.xml.parsers.SAXParserFactory")
        .or_else(|| config.get("javax.xml.parsers.SAXParserFactory").cloned());
    Ok(if provider.as_deref() == Some("com.ctc.wstx.sax.WstxSAXParserFactory") {
        SaxProvider::Woodstox
    } else {
        SaxProvider::Jdk
    })
}

/// KsmlSourceIndex enables disallow-doctype-decl before parsing. Woodstox
/// rejects that feature, and the source catches the resulting SAX exception.
pub(crate) fn source_index_available() -> Result<bool, XmlFactoryError> {
    if sax_provider()? == SaxProvider::Woodstox { return Ok(false); }
    entity_expansion_limit(ParserFactory::Sax)?;
    Ok(true)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FactoryCause {
    class: String,
    message: Option<String>,
}

/// The source raises an Error here, outside its recoverable XML diagnostics.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct XmlFactoryError {
    exception_class: String,
    message: Option<String>,
    bad_request: bool,
    causes: Vec<FactoryCause>,
}

impl XmlFactoryError {
    pub fn local_uncaught_msv(self) -> Self {
        if !self.recursive_msv_class_init() {
            return self;
        }
        let class = "java.lang.NoClassDefFoundError".to_owned();
        let message = "Could not initialize class java.lang.invoke.StringConcatFactory$InlineHiddenClassStrategy".to_owned();
        Self {
            exception_class: class.clone(),
            message: Some(message.clone()),
            bad_request: false,
            causes: vec![FactoryCause { class, message: Some(message) }],
        }
    }

    pub fn local_preview_msv(&self) -> Option<String> {
        self.recursive_msv_class_init().then(||
            "java.lang.NoClassDefFoundError: Could not initialize class java.lang.invoke.StringConcatFactory".to_owned())
    }

    fn recursive_msv_class_init(&self) -> bool {
        matches!(self.message.as_deref(),
            Some("Could not initialize class java.lang.invoke.StringConcatFactory"
            | "Could not initialize class javax.xml.catalog.CatalogMessages"
            | "bootstrap method initialization exception"))
            && matches!(self.exception_class.as_str(),
                "java.lang.NoClassDefFoundError" | "java.lang.BootstrapMethodError")
    }

    pub(crate) fn catalog_after_recursive_msv(&self) -> Option<Self> {
        if self.exception_class != "java.lang.NoClassDefFoundError"
            || self.message.as_deref() != Some("Could not initialize class java.lang.invoke.StringConcatFactory")
        {
            return None;
        }
        let message = "Could not initialize class javax.xml.catalog.CatalogMessages".to_owned();
        Some(Self {
            exception_class: self.exception_class.clone(),
            message: Some(message.clone()),
            bad_request: false,
            causes: vec![FactoryCause {
                class: self.exception_class.clone(),
                message: Some(message),
            }],
        })
    }

    pub(crate) fn illegal_argument(message: String) -> Self {
        Self {
            exception_class: "java.lang.IllegalArgumentException".into(),
            causes: vec![FactoryCause { class: "java.lang.IllegalArgumentException".into(), message: Some(message.clone()) }],
            message: Some(message),
            bad_request: true,
        }
    }
    fn invalid_limit(factory: ParserFactory, property: &str) -> Self {
        Self::wrapped(factory, "java.lang.NumberFormatException", format!("Invalid setting for system property: {property}"))
    }

    fn configuration_error(factory: ParserFactory) -> Self {
        let message = "Malformed \\uxxxx encoding.".to_owned();
        match factory {
            ParserFactory::Dom => Self::wrapped(factory, "java.lang.IllegalArgumentException", message),
            // SAXParserFactory.newInstance looks up configuration before reflective construction.
            ParserFactory::Sax => Self {
                exception_class: "java.lang.IllegalArgumentException".into(),
                message: Some(message.clone()),
                bad_request: true,
                causes: vec![FactoryCause { class: "java.lang.IllegalArgumentException".into(), message: Some(message) }],
            },
        }
    }

    fn wrapped(factory: ParserFactory, cause: &'static str, cause_message: String) -> Self {
        let provider = match factory {
            ParserFactory::Dom => "DocumentBuilderFactoryImpl",
            ParserFactory::Sax => "SAXParserFactoryImpl",
        };
        let message = format!("Provider com.sun.org.apache.xerces.internal.jaxp.{provider} could not be instantiated: java.lang.reflect.InvocationTargetException");
        Self {
            exception_class: "javax.xml.parsers.FactoryConfigurationError".into(),
            message: Some(message.clone()),
            bad_request: false,
            causes: vec![
                FactoryCause {
                    class: "javax.xml.parsers.FactoryConfigurationError".into(),
                    message: Some(message),
                },
                FactoryCause {
                    class: "java.lang.reflect.InvocationTargetException".into(),
                    message: None,
                },
                FactoryCause {
                    class: cause.into(),
                    message: Some(cause_message),
                },
            ],
        }
    }

    pub fn is_bad_request(&self) -> bool {
        self.bad_request
    }

    pub fn is_internal_model_charset_failure(&self) -> bool {
        self.exception_class == "java.lang.reflect.UndeclaredThrowableException"
            && self.message.is_none()
    }

    /// BaseDomainManager reads its bundled UTF-8 XML as the JVM default
    /// charset. UTF-16 or UTF-32 decoding turns the opening `<?` into a
    /// non-XML code point before the uploaded model can be parsed.
    fn internal_model_charset_failure(charset: &str) -> Option<Self> {
        let first = match charset.to_ascii_uppercase().as_str() {
            "UTF-16" | "UTF-16BE" => '㰿',
            "UTF-16LE" => '㼼',
            "UTF-32" | "UTF-32LE" | "UTF-32BE" => '\u{fffd}',
            _ => return None,
        };
        let message = format!("Unexpected character '{first}' (code {} / 0x{:x}) in prolog; expected '<'\n at [row,col {{unknown-source}}]: [1,1]", first as u32, first as u32);
        Some(Self {
            exception_class: "java.lang.reflect.UndeclaredThrowableException".into(),
            message: None,
            bad_request: false,
            causes: vec![
                FactoryCause { class: "java.lang.reflect.UndeclaredThrowableException".into(), message: None },
                FactoryCause { class: "com.fasterxml.jackson.core.JsonParseException".into(), message: Some(message.clone()) },
                FactoryCause { class: "com.ctc.wstx.exc.WstxUnexpectedCharException".into(), message: Some(message) },
            ],
        })
    }

    pub fn servlet_message(&self) -> String {
        let exception = self.message.as_ref().map_or_else(
            || self.exception_class.clone(),
            |message| format!("{}: {message}", self.exception_class),
        );
        format!("Handler dispatch failed; nested exception is {exception}")
    }
}

impl std::fmt::Display for XmlFactoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message.as_deref().unwrap_or(&self.exception_class))
    }
}

impl std::error::Error for XmlFactoryError {}

pub(crate) fn entity_expansion_limit(factory: ParserFactory) -> Result<u64, XmlFactoryError> {
    Ok(SecurityLimits::load(factory)?.get(Limit::Expansion))
}

/// MVC creates its XML transformer before accepting requests. That lookup
/// loads configuration syntax without validating the parser's numeric limits.
pub fn check_server_configuration() -> Result<(), XmlFactoryError> {
    configuration().map(|_| ()).map_err(|()| XmlFactoryError::configuration_error(ParserFactory::Sax))
}

/// BaseDomainManager creates an XmlMapper for its built-in type definitions
/// before selecting the byte-stream SAX provider for the uploaded model.
pub(crate) fn check_mapper_configuration() -> Result<(), XmlFactoryError> {
    configuration().map_err(|()| XmlFactoryError::configuration_error(ParserFactory::Sax))?;
    if let Some(charset) = crate::process_properties::property("file.encoding") {
        if let Some(error) = XmlFactoryError::internal_model_charset_failure(&charset) {
            return Err(error);
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) enum Limit {
    Expansion,
    Occur,
    Attributes,
    TotalSize,
    GeneralSize,
    ParameterSize,
    Depth,
    Name,
    Replacement,
    XPathGroup,
    XPathOp,
    XPathTotal,
}

const LIMITS: [(Limit, &str, Option<&str>, u64); 12] = [
    (
        Limit::Expansion,
        "jdk.xml.entityExpansionLimit",
        Some("entityExpansionLimit"),
        2500,
    ),
    (
        Limit::Occur,
        "jdk.xml.maxOccurLimit",
        Some("maxOccurLimit"),
        5000,
    ),
    (
        Limit::Attributes,
        "jdk.xml.elementAttributeLimit",
        Some("elementAttributeLimit"),
        200,
    ),
    (
        Limit::TotalSize,
        "jdk.xml.totalEntitySizeLimit",
        None,
        100000,
    ),
    (
        Limit::GeneralSize,
        "jdk.xml.maxGeneralEntitySizeLimit",
        None,
        100000,
    ),
    (
        Limit::ParameterSize,
        "jdk.xml.maxParameterEntitySizeLimit",
        None,
        15000,
    ),
    (Limit::Depth, "jdk.xml.maxElementDepth", None, 100),
    (Limit::Name, "jdk.xml.maxXMLNameLimit", None, 1000),
    (
        Limit::Replacement,
        "jdk.xml.entityReplacementLimit",
        None,
        100000,
    ),
    (Limit::XPathGroup, "jdk.xml.xpathExprGrpLimit", None, 10),
    (Limit::XPathOp, "jdk.xml.xpathExprOpLimit", None, 100),
    (Limit::XPathTotal, "jdk.xml.xpathTotalOpLimit", None, 10000),
];

#[derive(Clone)]
pub(crate) struct SecurityLimits {
    values: [u64; 12],
    states: [&'static str; 12],
}

impl SecurityLimits {
    pub(crate) fn load(factory: ParserFactory) -> Result<Self, XmlFactoryError> {
        let config = factory_configuration(factory)?;
        if matches!(factory, ParserFactory::Sax) && sax_provider()? == SaxProvider::Woodstox {
            // These are Woodstox's own defaults. JDK properties are not read
            // by its constructor; JDK-only accounting remains disabled.
            let mut values = [0; 12];
            values[Limit::Expansion as usize] = 100000;
            values[Limit::Attributes as usize] = 1000;
            values[Limit::Depth as usize] = 1000;
            return Ok(Self { values, states: ["Woodstox default"; 12] });
        }
        let mut limits = Self {
            values: [0; 12],
            states: ["FEATURE_SECURE_PROCESSING"; 12],
        };
        for (limit, property, legacy, default) in LIMITS {
            let system = crate::process_properties::property(property)
                .filter(|v| !v.is_empty())
                .or_else(|| {
                    legacy
                        .and_then(crate::process_properties::property)
                        .filter(|v| !v.is_empty())
                });
            let setting = system.as_deref().or_else(|| {
                config
                    .get(property)
                    .map(String::as_str)
                    .filter(|v| !v.is_empty())
            });
            limits.values[limit as usize] = match setting {
                Some(value) => crate::hutool_json::parse_integer(value)
                    .and_then(|n| i32::try_from(n).ok())
                    .map(|n| n.max(0) as u64)
                    .ok_or_else(|| XmlFactoryError::invalid_limit(factory, property))?,
                None => default,
            };
            if system.is_some() {
                limits.states[limit as usize] = "system property";
            } else if setting.is_some() {
                limits.states[limit as usize] = "jaxp.properties";
            }
        }
        Ok(limits)
    }
    pub(crate) fn get(&self, limit: Limit) -> u64 {
        self.values[limit as usize]
    }
    pub(crate) fn state(&self, limit: Limit) -> &'static str {
        self.states[limit as usize]
    }
    pub(crate) fn over(&self, limit: Limit, value: u64) -> bool {
        self.get(limit) != 0 && value > self.get(limit)
    }
}

fn factory_configuration(factory: ParserFactory) -> Result<&'static std::collections::BTreeMap<String, String>, XmlFactoryError> {
    const KEY: &str = "javax.xml.parsers.SAXParserFactory";
    if matches!(factory, ParserFactory::Sax) {
        if let Some(provider) = crate::process_properties::property(KEY) {
            // Even an empty system value overrides the configuration file.
            providers::validate_sax(&provider)?;
            return configuration().map_err(|()| XmlFactoryError::wrapped(factory, "java.lang.IllegalArgumentException", "Malformed \\uxxxx encoding.".into()));
        }
        let config = configuration().map_err(|()| XmlFactoryError::configuration_error(factory))?;
        if let Some(provider) = config.get(KEY) { providers::validate_sax(provider)?; }
        Ok(config)
    } else {
        // Hutool passes its built-in DOM class explicitly; the DOM system and
        // configuration provider settings do not participate in that lookup.
        configuration().map_err(|()| XmlFactoryError::configuration_error(factory))
    }
}

fn configuration() -> Result<&'static std::collections::BTreeMap<String, String>, ()> {
    static CONFIG: std::sync::OnceLock<std::collections::BTreeMap<String, String>> =
        std::sync::OnceLock::new();
    static LOADING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    if let Some(config) = CONFIG.get() { return Ok(config); }
    let _loading = LOADING.lock().expect("XML configuration loading lock");
    if let Some(config) = CONFIG.get() { return Ok(config); }
    // JdkXmlConfig publishes only a successfully loaded configuration. A
    // malformed file is retried on the next factory creation.
    let result = {
        let mut result = std::collections::BTreeMap::new();
        let home = crate::process_properties::property("java.home")
            .or_else(|| std::env::var("JAVA_HOME").ok());
        let paths = home
            .map(|home| std::path::PathBuf::from(home).join("conf/jaxp.properties"))
            .into_iter()
            .chain(
                crate::process_properties::property("java.xml.config.file")
                    .map(std::path::PathBuf::from),
            );
        for path in paths {
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            let text = bytes
                .into_iter()
                .map(char::from)
                .collect::<String>()
                .replace("\r\n", "\n")
                .replace('\r', "\n");
            let mut pending = String::new();
            for line in text.split_terminator('\n') {
                let line = line.trim_start_matches([' ', '\t', '\u{c}']);
                // A comment is one natural line, even when it ends in '\\'.
                // A comment marker within a nonempty continued value is data.
                if pending.is_empty() && line.starts_with(['#', '!']) { continue; }
                pending.push_str(line);
                if pending
                    .as_bytes()
                    .iter()
                    .rev()
                    .take_while(|b| **b == b'\\')
                    .count()
                    % 2
                    == 1
                {
                    pending.pop();
                    continue;
                }
                property_line(&pending, &mut result)?;
                pending.clear();
            }
            // EOF also terminates a continued logical line; its final
            // unescaped backslash has already been removed above.
            property_line(&pending, &mut result)?;
        }
        result
    };
    Ok(CONFIG.get_or_init(|| result))
}

fn property_line(line: &str, result: &mut std::collections::BTreeMap<String, String>) -> Result<(), ()> {
    if line.is_empty() { return Ok(()); }
    let mut escaped = false;
    let end = line.char_indices().find_map(|(i, ch)| {
        if escaped { escaped = false; None }
        else if ch == '\\' { escaped = true; None }
        else if matches!(ch, '=' | ':' | ' ' | '\t' | '\u{c}') { Some(i) }
        else { None }
    }).unwrap_or(line.len());
    let rest = line[end..].trim_start_matches([' ', '\t', '\u{c}']);
    let value = rest.strip_prefix(['=', ':']).unwrap_or(rest).trim_start_matches([' ', '\t', '\u{c}']);
    result.insert(property_escapes(&line[..end])?, property_escapes(value)?);
    Ok(())
}

fn property_escapes(value: &str) -> Result<String, ()> {
    let mut out = Vec::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.extend(ch.encode_utf16(&mut [0; 2]).iter().copied());
            continue;
        }
        let Some(next) = chars.next() else { break };
        if next == 'u' {
            let hex = chars.by_ref().take(4).collect::<String>();
            if hex.len() != 4 || !hex.bytes().all(|ch| ch.is_ascii_hexdigit()) { return Err(()); }
            out.push(u16::from_str_radix(&hex, 16).map_err(|_| ())?);
        } else {
            let next = match next {
                't' => '\t',
                'r' => '\r',
                'n' => '\n',
                'f' => '\u{c}',
                other => other,
            };
            out.extend(next.encode_utf16(&mut [0; 2]).iter().copied());
        }
    }
    Ok(String::from_utf16_lossy(&out))
}
