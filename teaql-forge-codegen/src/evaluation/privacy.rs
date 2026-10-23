use super::{EvaluationItem, EvaluationResponse, PrivacyFinding};
use roxmltree::Node;
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubjectClass {
    HighRisk,
    Person,
    Account,
    Organization,
    Place,
    Asset,
    System,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldClass {
    ProhibitedAuth,
    RawPassword,
    Secret,
    Payment,
    GovernmentId,
    Contact,
    Personal,
    Location,
    Financial,
    Health,
    Communication,
    WeakText,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
    Suggestion,
    None,
}

impl Severity {
    fn rank(self) -> u8 {
        match self {
            Self::Error => 3,
            Self::Warning => 2,
            Self::Suggestion => 1,
            Self::None => 0,
        }
    }

    fn capped(self, maximum: Self) -> Self {
        if self.rank() > maximum.rank() {
            maximum
        } else {
            self
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Suggestion => "Suggestion",
            Self::None => "None",
        }
    }
}

struct SubjectSignal {
    class: SubjectClass,
    base: f64,
    multiplier: f64,
}

struct FieldSignal {
    class: FieldClass,
    score: f64,
    maximum: Severity,
    evidence: &'static [&'static str],
}

struct Decision {
    severity: Severity,
    class: FieldClass,
    reason: String,
    evidence: &'static [&'static str],
}

fn normalize(name: &str) -> String {
    let mut result = String::new();
    let mut previous = None;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if character.is_ascii_uppercase()
                && previous
                    .is_some_and(|value: char| value.is_ascii_lowercase() || value.is_ascii_digit())
                && !result.ends_with('_')
            {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
        } else if !result.is_empty() && !result.ends_with('_') {
            result.push('_');
        }
        previous = Some(character);
    }
    result.trim_matches('_').to_string()
}

fn phrase_match(compact: &str, phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| phrase.replace('_', "") == compact)
}

fn token_or_phrase(name: &str, words: &[&str], phrases: &[&str]) -> bool {
    let compact = name.replace('_', "");
    name.split('_').any(|token| words.contains(&token)) || phrase_match(&compact, phrases)
}

fn subject(name: &str) -> SubjectSignal {
    let compact = normalize(name).replace('_', "");
    let (class, base, multiplier) = if phrase_match(
        &compact,
        &[
            "credit_card",
            "payment_card",
            "bank_account",
            "credential",
            "api_key",
            "api_credential",
            "auth_token",
            "access_token",
            "secret",
            "medical_record",
            "identity_document",
            "passport",
            "id_card",
            "payment_account",
            "financial_account",
            "health_record",
            "patient_record",
            "payment_method",
        ],
    ) {
        (SubjectClass::HighRisk, 80.0, 1.0)
    } else if phrase_match(
        &compact,
        &[
            "student",
            "customer",
            "employee",
            "patient",
            "user",
            "teacher",
            "member",
            "applicant",
            "contact",
            "driver",
            "passenger",
            "doctor",
            "parent",
            "guardian",
            "person",
            "account_holder",
            "customer_contact",
            "team_member",
            "staff_member",
        ],
    ) {
        (SubjectClass::Person, 15.0, 1.4)
    } else if phrase_match(
        &compact,
        &["account", "login", "user_account", "session", "profile"],
    ) {
        (SubjectClass::Account, 25.0, 1.5)
    } else if phrase_match(
        &compact,
        &[
            "school",
            "company",
            "organization",
            "department",
            "store",
            "merchant",
            "vendor",
            "supplier",
            "hospital",
            "bank",
            "agency",
            "shop",
        ],
    ) {
        (SubjectClass::Organization, 0.0, 1.0)
    } else if phrase_match(
        &compact,
        &[
            "warehouse",
            "branch",
            "campus",
            "room",
            "location",
            "address",
            "office",
        ],
    ) {
        (SubjectClass::Place, 0.0, 1.0)
    } else if phrase_match(
        &compact,
        &[
            "product",
            "course",
            "class",
            "category",
            "device",
            "vehicle",
            "asset",
            "equipment",
        ],
    ) {
        (SubjectClass::Asset, 0.0, 1.0)
    } else if phrase_match(
        &compact,
        &[
            "role",
            "permission",
            "config",
            "status",
            "type",
            "log",
            "event",
            "job",
            "task",
            "user_role",
            "team_member_role",
            "customer_type",
            "account_type",
            "customer_status",
            "account_chart",
            "chart_of_account",
            "chart_of_accounts",
        ],
    ) {
        (SubjectClass::System, 0.0, 1.0)
    } else {
        (SubjectClass::Unknown, 0.0, 1.0)
    };
    SubjectSignal {
        class,
        base,
        multiplier,
    }
}

fn field(name: &str, object_name: &str) -> FieldSignal {
    let normalized = normalize(name);
    let compact = normalized.replace('_', "");
    let payment_auth_context = phrase_match(
        &normalize(object_name).replace('_', ""),
        &[
            "credit_card",
            "payment_card",
            "debit_card",
            "bank_card",
            "payment_method",
            "card_authentication",
            "payment_card_authentication",
        ],
    );
    let signal = |class, score, maximum, evidence| FieldSignal {
        class,
        score,
        maximum,
        evidence,
    };
    if token_or_phrase(
        &normalized,
        &["cvv", "cvv2", "cvc", "cvc2", "cav2", "cvn2"],
        &[
            "card_security_code",
            "card_verification_code",
            "card_verification_value",
            "card_validation_code",
            "card_validation_value",
            "payment_pin",
            "card_pin",
            "pin_block",
            "encrypted_pin_block",
            "full_track_data",
            "track_1_data",
            "track_2_data",
            "magnetic_stripe_data",
            "chip_equivalent_data",
            "temporary_password",
            "one_time_password",
        ],
    ) || (payment_auth_context && token_or_phrase(&normalized, &["cid", "pin"], &[]))
    {
        return signal(FieldClass::ProhibitedAuth, 100.0, Severity::Error, &[
            "PCI DSS Requirement 3.3.1: sensitive authentication data must not be stored after authorization",
        ]);
    }
    if phrase_match(
        &compact,
        &[
            "password",
            "passwd",
            "pwd",
            "user_password",
            "login_password",
        ],
    ) {
        return signal(FieldClass::RawPassword, 100.0, Severity::Error, &[
            "PCI DSS v4.0.1 Requirement 8.3.2: render authentication factors unreadable during transmission and storage using strong cryptography",
        ]);
    }
    if token_or_phrase(
        &normalized,
        &["token", "secret", "passphrase", "salt", "jwt", "credential"],
        &[
            "api_key",
            "auth_key",
            "access_token",
            "auth_token",
            "refresh_token",
            "session_token",
            "private_key",
            "secret_key",
            "password_hash",
            "password_salt",
            "client_secret",
            "app_secret",
            "oauth_token",
            "bearer_token",
        ],
    ) {
        return signal(
            FieldClass::Secret,
            95.0,
            Severity::Error,
            &["CCPA/CPRA 1798.140: account access credentials"],
        );
    }
    if token_or_phrase(
        &normalized,
        &["ssn", "passport"],
        &[
            "social_security",
            "id_card",
            "idcard",
            "national_id",
            "passport_number",
            "id_card_number",
            "idcard_number",
            "national_id_number",
            "driver_license",
            "driver_license_number",
            "tax_id",
            "taxpayer_id",
        ],
    ) {
        return signal(
            FieldClass::GovernmentId,
            75.0,
            Severity::Warning,
            &[
                "CCPA/CPRA 1798.140: government identifiers",
                "PIPL Art. 28: specific identity",
            ],
        );
    }
    if token_or_phrase(
        &normalized,
        &["ccnum", "pan"],
        &[
            "credit_card",
            "credit_card_number",
            "card_number",
            "cardholder_number",
            "card_expiry_date",
            "credit_card_expiry_date",
            "payment_card_expiry_date",
            "debit_card_expiry_date",
            "card_expiration_date",
            "routing_number",
        ],
    ) {
        return signal(
            FieldClass::Payment,
            95.0,
            Severity::Error,
            &[
                "CCPA/CPRA 1798.140: financial account and card data",
                "PIPL Art. 28: financial accounts",
            ],
        );
    }
    if phrase_match(
        &compact,
        &["expiry_date", "expiration_date", "last_four_digits"],
    ) {
        return signal(
            FieldClass::Payment,
            40.0,
            Severity::Error,
            &[
                "CCPA/CPRA 1798.140: financial account and card data",
                "PIPL Art. 28: financial accounts",
            ],
        );
    }
    if token_or_phrase(
        &normalized,
        &[
            "health",
            "medical",
            "patient",
            "diagnosis",
            "treatment",
            "prescription",
        ],
        &["medical_record", "health_record"],
    ) {
        return signal(
            FieldClass::Health,
            80.0,
            Severity::Warning,
            &[
                "GDPR Art. 9: health data",
                "PIPL Art. 28: medical health",
                "CCPA/CPRA 1798.140: health information",
            ],
        );
    }
    if token_or_phrase(
        &normalized,
        &["salary", "income", "balance", "iban", "swift"],
        &[
            "bank_account",
            "payment_account",
            "financial_account",
            "account_number",
            "bank_account_number",
            "payment_account_number",
            "iban_code",
            "swift_code",
        ],
    ) {
        return signal(
            FieldClass::Financial,
            60.0,
            Severity::Warning,
            &[
                "PIPL Art. 28: financial accounts",
                "CCPA/CPRA 1798.140: financial information",
            ],
        );
    }
    if token_or_phrase(
        &normalized,
        &["phone", "tel", "email"],
        &[
            "phone_number",
            "mobile_phone",
            "mobile_number",
            "cell_phone",
            "cell_number",
            "email_address",
            "personal_email",
            "work_email",
        ],
    ) {
        return signal(
            FieldClass::Contact,
            45.0,
            Severity::Warning,
            &["CCPA/CPRA 1798.140: identifiers"],
        );
    }
    if token_or_phrase(
        &normalized,
        &[
            "address",
            "location",
            "gps",
            "latitude",
            "longitude",
            "coord",
        ],
        &[
            "home_address",
            "street_address",
            "gps_coordinate",
            "geo_location",
            "precise_location",
        ],
    ) {
        return signal(
            FieldClass::Location,
            35.0,
            Severity::Warning,
            &[
                "CCPA/CPRA 1798.140: precise geolocation",
                "PIPL Art. 28: whereabouts",
            ],
        );
    }
    if token_or_phrase(
        &normalized,
        &["name", "gender", "sex", "age", "birth", "dob"],
        &[
            "first_name",
            "firstname",
            "last_name",
            "lastname",
            "full_name",
            "user_name",
            "username",
            "birth_date",
            "date_of_birth",
        ],
    ) {
        return signal(
            FieldClass::Personal,
            20.0,
            Severity::Suggestion,
            &["GDPR Art. 4: identifier"],
        );
    }
    if token_or_phrase(
        &normalized,
        &["chat", "message", "conversation", "history"],
        &[
            "chat_history",
            "message_history",
            "conversation_history",
            "email_content",
        ],
    ) {
        return signal(
            FieldClass::Communication,
            35.0,
            Severity::Suggestion,
            &["CCPA/CPRA 1798.140: communications content"],
        );
    }
    if token_or_phrase(
        &normalized,
        &["note", "remark", "description", "comment", "memo"],
        &[],
    ) {
        return signal(
            FieldClass::WeakText,
            5.0,
            Severity::Suggestion,
            &["High-risk subject context"],
        );
    }
    signal(FieldClass::None, 0.0, Severity::None, &[])
}

fn decision(subject: SubjectSignal, field: FieldSignal) -> Decision {
    if field.class == FieldClass::None {
        return Decision {
            severity: Severity::None,
            class: field.class,
            reason: String::new(),
            evidence: field.evidence,
        };
    }
    if field.class == FieldClass::ProhibitedAuth || field.class == FieldClass::RawPassword {
        return Decision {
            severity: Severity::Error,
            class: field.class,
            reason: if field.class == FieldClass::ProhibitedAuth {
                "Temporary authentication data must not be modeled or persisted."
            } else {
                "A password field implies plaintext storage and must be modeled as a password hash."
            }
            .into(),
            evidence: field.evidence,
        };
    }
    let adjustment = match (subject.class, field.class) {
        (SubjectClass::Person, FieldClass::Contact) => 15.0,
        (SubjectClass::Person, FieldClass::Location) => 20.0,
        (SubjectClass::Person, FieldClass::Personal) => 15.0,
        (SubjectClass::Account, FieldClass::Secret) => 10.0,
        (SubjectClass::HighRisk, FieldClass::WeakText) => 5.0,
        (
            SubjectClass::Organization
            | SubjectClass::Place
            | SubjectClass::Asset
            | SubjectClass::System,
            FieldClass::Personal,
        ) => -25.0,
        (SubjectClass::Organization, FieldClass::Contact) => -25.0,
        (SubjectClass::Place, FieldClass::Location) => -25.0,
        (SubjectClass::Asset, FieldClass::Location) => -20.0,
        _ => 0.0,
    };
    let score = (subject.base + field.score * subject.multiplier + adjustment).round() as i32;
    let mut severity = if score >= 90 {
        Severity::Error
    } else if score >= 65 {
        Severity::Warning
    } else if score >= 45 {
        Severity::Suggestion
    } else {
        Severity::None
    };
    let mut maximum = field.maximum;
    if subject.class == SubjectClass::HighRisk {
        if matches!(
            field.class,
            FieldClass::WeakText | FieldClass::Personal | FieldClass::Communication
        ) {
            maximum = Severity::Warning;
        }
        if severity.rank() < Severity::Warning.rank() {
            severity = Severity::Warning;
        }
    }
    severity = severity.capped(maximum);
    let label = match field.class {
        FieldClass::ProhibitedAuth => "Prohibited authentication data",
        FieldClass::RawPassword => "Plaintext password",
        FieldClass::Secret => "Secret credential",
        FieldClass::Payment => "Payment data",
        FieldClass::GovernmentId => "Government identifier",
        FieldClass::Contact => "Contact identifier",
        FieldClass::Personal => "Personal attribute",
        FieldClass::Location => "Location data",
        FieldClass::Financial => "Financial data",
        FieldClass::Health => "Health data",
        FieldClass::Communication => "Communication data",
        FieldClass::WeakText => "Free-text",
        FieldClass::None => "Privacy-sensitive",
    };
    let reason = match (subject.class, field.class) {
        (SubjectClass::HighRisk, FieldClass::WeakText) => {
            "Weak text field on high-risk privacy subject.".into()
        }
        (SubjectClass::HighRisk, _) => format!("{label} on high-risk privacy subject."),
        (SubjectClass::Person, _) => format!("{label} on person-like subject."),
        _ => format!("{label} field."),
    };
    Decision {
        severity,
        class: field.class,
        reason,
        evidence: field.evidence,
    }
}

fn requires_model_change(class: FieldClass) -> bool {
    matches!(class, FieldClass::ProhibitedAuth | FieldClass::RawPassword)
}

fn report_finding(
    object: Node<'_, '_>,
    field_name: &str,
    mask_config: &str,
    decision: Decision,
    filename: &str,
    response: &mut EvaluationResponse,
) {
    let object_name = crate::java_xml::qname(object);
    let masks = if mask_config.is_empty() {
        field_name.to_string()
    } else {
        format!("{mask_config},{field_name}")
    };
    let (action, recommendation) = match decision.class {
        FieldClass::ProhibitedAuth => (
            format!("Delete field `{field_name}` from the KSML model; masking or encryption is not an acceptable fix."),
            format!("Delete field `{field_name}` from the KSML model."),
        ),
        FieldClass::RawPassword => {
            let normalized = normalize(field_name);
            let replacement = if normalized.ends_with("_password") {
                format!("{normalized}_hash")
            } else {
                "password_hash".into()
            };
            (
                format!("Rename field `{field_name}` to `{replacement}` and store only a non-reversible password hash; then add `{replacement}` to `_audit_mask_fields`."),
                format!("Rename to `{replacement}`, store only a password hash, and mask it in audits."),
            )
        }
        _ => (
            format!("Add `{field_name}` to `_audit_mask_fields` (e.g., `<{object_name} _audit_mask_fields=\"{masks}\">`)."),
            format!("Add to `_audit_mask_fields=\"{masks}\"`."),
        ),
    };
    let evidence_limit = match decision.severity {
        Severity::Error => 3,
        Severity::Warning => 2,
        Severity::Suggestion | Severity::None => 1,
    };
    let basis = decision
        .evidence
        .iter()
        .take(evidence_limit)
        .copied()
        .collect::<Vec<_>>()
        .join("; ");
    let mut message = decision.reason.clone();
    if requires_model_change(decision.class) {
        message.push_str(&format!(" Regulatory basis: {basis}."));
    }
    message.push(' ');
    message.push_str(&action);
    let (rule_id, title) = match decision.class {
        FieldClass::ProhibitedAuth => (
            "KSML-PRIVACY-001-PROHIBITED",
            "Prohibited authentication data in persistent model",
        ),
        FieldClass::RawPassword => (
            "KSML-PRIVACY-001-PASSWORD",
            "Plaintext password field in persistent model",
        ),
        _ => match decision.severity {
            Severity::Error => ("KSML-PRIVACY-001-ERR", "High sensitivity data unmasked"),
            Severity::Warning => ("KSML-PRIVACY-001-WARN", "Privacy data masking warning"),
            Severity::Suggestion => ("KSML-PRIVACY-001-SUG", "Potential personal data suggestion"),
            Severity::None => return,
        },
    };
    let source = object.document().input_text();
    let start = object.range().start;
    let tag_end = start + source[start..].find('>').unwrap_or(0);
    let finding = EvaluationItem {
        rule_id: rule_id.into(),
        title: title.into(),
        message,
        path: format!("/root/{object_name}/{field_name}"),
        object_name: Some(object_name.into()),
        field_name: Some(field_name.into()),
        xml_path: Some(filename.into()),
        line_number: Some(object.document().text_pos_at(tag_end).row as usize),
    };
    match decision.severity {
        Severity::Error => response.errors.push(finding),
        Severity::Warning => response.warnings.push(finding),
        Severity::Suggestion => response.suggestions.push(finding),
        Severity::None => {}
    }
    response.privacy_findings.push(PrivacyFinding {
        severity: decision.severity.label().into(),
        target: format!("{object_name}.{field_name}"),
        reason: decision.reason,
        recommended_action: recommendation,
        regulatory_basis: basis,
    });
}

pub(super) fn evaluate(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    for object in root
        .children()
        .filter(|node| node.is_element() && !crate::java_xml::qname(node).starts_with('_'))
    {
        if object.attribute("_deleted") == Some("true") {
            continue;
        }
        let object_name = crate::java_xml::qname(object);
        let mask_config = object.attribute("_audit_mask_fields").unwrap_or("");
        let masks = mask_config
            .split(',')
            .map(normalize)
            .filter(|field| !field.is_empty())
            .collect::<HashSet<_>>();
        let mut fields = object
            .attributes()
            .filter(|attribute| !attribute.name().starts_with('_'))
            .map(|attribute| normalize(attribute.name()))
            .collect::<HashSet<_>>();
        fields.insert("id".into());
        fields.insert("version".into());
        let mut ordered_masks = mask_config
            .split(',')
            .map(normalize)
            .filter(|field| !field.is_empty())
            .fold(Vec::new(), |mut fields, field| {
                if !fields.contains(&field) {
                    fields.push(field);
                }
                fields
            });
        let mut capacity = 16u32;
        while ordered_masks.len() > capacity as usize * 3 / 4 {
            capacity *= 2;
        }
        ordered_masks.sort_by_key(|field| {
            let hash = field.encode_utf16().fold(0u32, |hash, character| {
                hash.wrapping_mul(31).wrapping_add(character as u32)
            });
            (hash ^ (hash >> 16)) & (capacity - 1)
        });
        for masked in &ordered_masks {
            if !fields.contains(masked) {
                response.errors.push(EvaluationItem {
                    rule_id: "KSML-PRIVACY-002-MASK-FIELD".into(),
                    title: "Unknown audit mask field".into(),
                    message: format!("`_audit_mask_fields` names `{masked}`, but `{object_name}` has no matching field. Correct the field name; otherwise a sensitive value may be logged as plain text."),
                    path: format!("/root/{object_name}/@_audit_mask_fields"),
                    object_name: Some(object_name.into()),
                    field_name: Some("_audit_mask_fields".into()),
                    xml_path: Some(filename.into()),
                    line_number: Some(super::rules::opening_tag_end_line(object)),
                });
            }
        }
        if object.attribute("_constant") == Some("true") {
            continue;
        }
        let subject_signal = subject(object_name);
        for attribute in object.attributes() {
            let name = attribute.name();
            if name.starts_with('_') {
                continue;
            }
            let normalized = normalize(name);
            if normalized.is_empty() {
                continue;
            }
            let signal = field(&normalized, object_name);
            if masks.contains(&normalized) && !requires_model_change(signal.class) {
                continue;
            }
            let decision = decision(
                SubjectSignal {
                    class: subject_signal.class,
                    base: subject_signal.base,
                    multiplier: subject_signal.multiplier,
                },
                signal,
            );
            if decision.severity != Severity::None {
                report_finding(object, name, mask_config, decision, filename, response);
            }
        }
    }
}
