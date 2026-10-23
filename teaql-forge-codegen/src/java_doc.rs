use crate::context::{RenderDomain, RenderEntity};

fn st4_rendered_line_endings(rendered: String) -> String {
    // ST4 drops bare CR while writing an interpolated value and writes CRLF
    // as LF. These document bodies are assembled without the ST4 writer.
    rendered.replace("\r\n", "\n").replace('\r', "")
}

pub(crate) fn render_with_descriptor_override(
    domain: &RenderDomain, source: &str,
) -> Result<Option<String>, crate::engine::GenerationError> {
    if !crate::preview_domain::has_descriptor_override(domain) { return Ok(None) }
    let (mut vars, mut graph) = crate::preview_graph::context(domain);
    if source == include_str!("../templates/doc-shadow-source/data-design.stg")
        || source == include_str!("../templates/doc-shadow-source/data-design-react.stg") {
        vars["domain"]=crate::graph_service::attach(domain,&mut graph);
    }
    crate::java_stg::StgTemplate::parse_preview(source)
        .and_then(|template| template.render_graph_preview(vars, graph))
        .map(|value| value.split('\n').map(|line| if line.trim().is_empty() { "" } else { line }).collect::<Vec<_>>().join("\n"))
        .map(Some).map_err(crate::engine::GenerationError::InvalidJavaModel)
}

struct Column {
    sql_name: String,
    name: String,
    sql_type: String,
    sample: String,
}

fn present_name(name: &str) -> String {
    name.split('_')
        .map(|part| {
            crate::preview_domain::upper_first(part)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sql_name(entity: &RenderEntity, name: &str) -> String {
    if let Some(column) = entity
        .fields
        .iter()
        .find(|field| field.name == name)
        .and_then(|field| field.attribute("column"))
        .or_else(|| {
            entity
                .object_fields
                .iter()
                .find(|field| field.name == name)
                .and_then(|field| field.metadata.get("column").map(String::as_str))
        })
    {
        return column.to_uppercase();
    }
    let name = crate::preview_domain::snake(name).to_uppercase();
    let keyword = include_str!("../templates/doc/SQL2016.kw")
        .split_whitespace()
        .any(|keyword| keyword.eq_ignore_ascii_case(&name));
    if keyword {
        let quote = entity
            .data_service_attributes
            .get("quoteCharForKeywords")
            .map(String::as_str)
            .unwrap_or("");
        format!("{quote}{name}{quote}")
    } else {
        name
    }
}

fn constraint_names(
    entity: &RenderEntity,
    constraint: &crate::context::RenderConstraint,
) -> (String, String) {
    let column = entity
        .object_fields
        .iter()
        .find(|relation| crate::preview_domain::snake(&relation.name).to_uppercase() == constraint.columns)
        .map(|relation| sql_name(entity, &relation.name))
        .unwrap_or_else(|| constraint.columns.clone());
    let name = constraint
        .name
        .strip_suffix(&constraint.columns)
        .map(|prefix| format!("{prefix}{column}"))
        .unwrap_or_else(|| constraint.name.clone());
    (name, column)
}

fn columns(domain: &RenderDomain, entity: &RenderEntity) -> Vec<Column> {
    let mut columns: Vec<_> = entity
        .fields
        .iter()
        .map(|field| {
            let sql_type = crate::sql::field_type(entity, field);
            let sample = field.attribute("defaultSampleValue").filter(|value| !value.is_empty())
                .map(|value| match crate::preview_domain::attribute_value(value) {
                    serde_json::Value::String(value) => value,
                    value => value.to_string(),
                }).or_else(|| field.metadata.get("candidates")
                .cloned())
                .or_else(|| field.metadata.get("sampleValue").cloned())
                .or_else(|| {
                    (!field.source_value.is_empty()).then(|| {
                        field
                            .source_value
                            .split('|')
                            .last()
                            .unwrap_or(&field.source_value)
                            .trim_end_matches('?')
                            .to_string()
                    })
                })
                .unwrap_or_else(|| "1".to_string());
            let sample = crate::preview_rust::candidate_sample(&sample, 1).to_string();
            (
                field.source_order,
                Column {
                    sql_name: sql_name(entity, &field.name),
                    name: present_name(&field.name),
                    sql_type,
                    sample,
                },
            )
        })
        .collect();
    columns.extend(entity.object_fields.iter().map(|relation| {
        (
            relation.source_order,
            Column {
                sql_name: sql_name(entity, &relation.name),
                name: present_name(&relation.name),
                sql_type: crate::sql::relation_type(domain, entity),
                sample: "1".into(),
            },
        )
    }));
    columns.sort_by_key(|item| item.0);
    columns.into_iter().map(|(_, column)| column).collect()
}

fn indexes(entity: &RenderEntity) -> Vec<(String, String, String, String)> {
    let prefix = entity.name.to_uppercase();
    let mut result = vec![
        (
            format!("PK_{prefix}_ID"),
            "ID".into(),
            "PRIMARY KEY".into(),
            String::new(),
        ),
        (
            format!("PK_{prefix}_ID_VERSION"),
            "ID,VERSION".into(),
            "UNIQUE KEY".into(),
            String::new(),
        ),
    ];
    for relation in &entity.object_fields {
        let col = sql_name(entity, &relation.name);
        result.push((
            format!("IDX_{prefix}_{col}"),
            col,
            "INDEX".into(),
            String::new(),
        ));
    }
    for field in &entity.fields {
        if field.is_date {
            let col = sql_name(entity, &field.name);
            result.push((
                format!("IDX_{prefix}_{col}"),
                col,
                "INDEX".into(),
                String::new(),
            ));
        }
    }
    result
}

fn react_table(domain: &RenderDomain, entity: &RenderEntity) -> String {
    let columns = columns(domain, entity);
    let indexes = indexes(entity);
    let mut output = format!("{{\n                    \"name\": \"{}\",\n                    \"conceptName\": \"{}\",\n                    \"relationCount\": {},\n                    \"columns\": [ ", entity.table_name.to_uppercase(), present_name(&entity.name), entity.reverse_relations.len());
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(&format!("{{\n                    \"sqlName\": \"{}\",\n                    \"name\": \"{}\",\n                    \"type\": \"{}\",\n                    \"sample\": \"{}\"\n                }}", column.sql_name, column.name, column.sql_type, crate::java_stg::inline_value(&column.sample, 16)));
    }
    output.push_str(" ],\n                    \"indexes\": [ ");
    for (index, (name, cols, ty, remark)) in indexes.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(&format!("{{\n                    \"name\": \"{name}\",\n                    \"columns\": \"{cols}\",\n                    \"type\": \"{ty}\",\n                    \"remark\": \"{remark}\"\n                }}"));
    }
    output.push_str(" ],\n                    \"constraints\": [ ");
    for (index, constraint) in entity.constraints.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        let ref_table = entity
            .object_fields
            .iter()
            .find(|relation| crate::preview_domain::snake(&relation.name).to_uppercase() == constraint.columns)
            .and_then(|relation| {
                domain
                    .entities
                    .iter()
                    .find(|target| target.rust_struct == relation.target_struct)
            })
            .map(|target| target.table_name.to_uppercase())
            .unwrap_or_else(|| constraint.ref_table.clone());
        output.push_str(&format!("{{\n                    \"name\": \"{}\",\n                    \"columns\": \"{}\",\n                    \"refTable\": \"{}\",\n                    \"refCols\": \"{}\",\n                    \"remark\": \"{}\"\n                }}", constraint_names(entity,constraint).0, constraint_names(entity,constraint).1, ref_table, constraint.ref_cols, constraint.remark));
    }
    output.push_str(" ]\n                }");
    output
}

pub(crate) fn render_data_design_react(domain: &RenderDomain) -> String {
    let tables = domain
        .entities
        .iter()
        .map(|entity| react_table(domain, entity))
        .collect::<Vec<_>>()
        .join(",\n                ");
    st4_rendered_line_endings(include_str!("../templates/doc/data-design-react-source.txt")
        .replace("__DOMAIN_NAME__", &domain.name)
        .replace("__TABLES__", &format!("                {tables}")))
}

fn table_definition(domain: &RenderDomain, entity: &RenderEntity, index: usize) -> String {
    let cols = columns(domain, entity);
    let col_rows = cols.iter().enumerate().map(|(i, col)| format!("                            <tr><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>", i + 1, col.sql_name, col.name, col.sql_type, crate::java_stg::inline_value(&col.sample, 28))).collect::<Vec<_>>().join("\n");
    let index_rows = indexes(entity).iter().enumerate().map(|(i, (name, cols, kind, remark))| format!("                            <tr><td>{}</td><td>{name}</td><td>{cols}</td><td>{kind}</td><td>{remark}</td></tr>", i + 1)).collect::<Vec<_>>().join("\n");
    let constraint_rows = entity.constraints.iter().enumerate().map(|(i, constraint)| {
        let ref_table = entity.object_fields.iter().find(|relation| crate::preview_domain::snake(&relation.name).to_uppercase() == constraint.columns)
            .and_then(|relation| domain.entities.iter().find(|target| target.rust_struct == relation.target_struct))
            .map(|target| target.table_name.to_uppercase()).unwrap_or_else(|| constraint.ref_table.clone());
        format!("                            <tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", i + 1, constraint_names(entity,constraint).0, constraint_names(entity,constraint).1, ref_table, constraint.ref_cols, constraint.remark)
    }).collect::<Vec<_>>().join("\n");
    let mut sections = Vec::new();
    if !entity.object_fields.is_empty() {
        let rows = entity.object_fields.iter().enumerate().filter_map(|(i, relation)| {
            let target = domain.entities.iter().find(|target| target.rust_struct == relation.target_struct)?;
            Some(format!("                            <tr><td>{}</td><td>{}</td><td>{}</td><td>-</td></tr>", i + 1, target.table_name.to_uppercase(), sql_name(entity,&relation.name)))
        }).collect::<Vec<_>>().join("\n");
        sections
            .push(include_str!("../templates/doc/depends-source.txt").replace("__ROWS__", &rows));
    }
    if !entity.reverse_relations.is_empty() {
        let rows = entity.reverse_relations.iter().enumerate().filter_map(|(i, relation)| {
            let child = domain.entities.iter().find(|candidate| candidate.rust_struct == relation.target_struct)?;
            let col = sql_name(child,relation.local_key.trim_end_matches("_id"));
            Some(format!("                            <tr><td>{}</td><td>{}</td><td>{}</td><td>-</td></tr>", i + 1, child.table_name.to_uppercase(), col))
        }).collect::<Vec<_>>().join("\n");
        sections.push(
            include_str!("../templates/doc/depended-by-source.txt").replace("__ROWS__", &rows),
        );
    }
    if entity.constant {
        let headers = cols
            .iter()
            .map(|col| format!("<th>{}</th>", col.sql_name))
            .collect::<String>();
        let rows = entity
            .seed_values
            .iter()
            .enumerate()
            .map(|(i, seed)| {
                let values = cols
                    .iter()
                    .map(|column| {
                        let field_name = column.sql_name.to_lowercase();
                        let value =
                            seed.properties
                                .get(&field_name)
                                .cloned()
                                .unwrap_or_else(|| {
                                    if field_name == "id" {
                                        (i + 1).to_string()
                                    } else {
                                        "1".into()
                                    }
                                });
                        format!("<td>{}</td>", crate::java_stg::inline_value(&value, 28))
                    })
                    .collect::<String>();
                format!("                            <tr>{values}</tr>")
            })
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(
            include_str!("../templates/doc/constant-source.txt")
                .replace("__HEADER__", &format!("<tr>{headers}</tr>"))
                .replace("__ROWS__", &rows),
        );
    }
    let mut out = include_str!("../templates/doc/table-definition-source.txt")
        .replace("__TABLE_NAME__", &entity.table_name.to_uppercase())
        .replace("__INDEX__", &(index + 1).to_string())
        .replace("__COLUMNS_ROWS__", &col_rows)
        .replace("__INDEX_ROWS__", &index_rows)
        .replace("__CONSTRAINTS_ROWS__", &constraint_rows);
    if sections.is_empty() {
        out = out.replace("\n__SECTIONS__\n", "");
    } else {
        out = out.replace("__SECTIONS__", &sections.join("\n\n"));
        if entity.constant {
            out = out.replace(
                "                    </table>\n\n                </div>",
                "                    </table>\n                </div>",
            );
        }
    }
    out
}

pub(crate) fn render_data_design(domain: &RenderDomain) -> String {
    let sidebar = domain.entities.iter().enumerate().map(|(i, entity)| {
        let name = entity.table_name.to_uppercase();
        format!("                <a href=\"#Table-{name}\" class=\"list-group-item list-group-item-action text-truncate\" title=\"{name}\">\n                    <small>{}. {name}</small>\n                </a>", i + 1)
    }).collect::<Vec<_>>().join("\n");
    let index = domain.entities.iter().enumerate().map(|(i, entity)| {
        let name = entity.table_name.to_uppercase();
        let property_count = if entity.metadata_ordered.iter().any(|(key, value)| key == "properties" && !value.is_empty()) {
            1 // ST4 length() observes the attached scalar value.
        } else {
            entity.fields.iter().filter(|field| !field.is_id && !field.is_version).count() + entity.object_fields.len()
        };
        format!("                    <tr>\n                        <td>{}</td>\n                        <td><a href=\"#Table-{name}\">{name}</a></td>\n                        <td>{}</td>\n                        <td>{}</td>\n                    </tr>", i + 1, property_count, entity.reverse_relations.len())
    }).collect::<Vec<_>>().join("\n");
    let tables = domain
        .entities
        .iter()
        .enumerate()
        .map(|(i, entity)| table_definition(domain, entity, i))
        .collect::<Vec<_>>()
        .join("");
    st4_rendered_line_endings(include_str!("../templates/doc/data-design-source.txt")
        .replace("__DOMAIN_NAME__", &domain.name)
        .replace("__SIDEBAR__", &sidebar)
        .replace("__INDEX__", &index)
        .replace("__TABLES__", &tables.trim_end_matches('\n'))
        .replace("__WORD_CLOUD_LINE__\n", &{
            let (mut vars,mut graph)=crate::preview_graph::context(domain);
            vars["domain"]=crate::graph_service::attach(domain,&mut graph);
            crate::java_stg::StgTemplate::parse_preview("                <domain.wordCloudSVG>\n")
                .and_then(|template| template.render_graph_preview(vars,graph))
                .expect("fixed word cloud expression")
        }))
}

fn node(entity: &RenderEntity, depth: usize) -> String {
    let mut fields: Vec<_> = entity
        .fields
        .iter()
        .filter(|field| !field.is_id && !field.is_version)
        .map(|field| {
            (
                field.source_order,
                field.column_name.clone(),
                field.required,
            )
        })
        .chain(entity.object_fields.iter().map(|relation| {
            (
                relation.source_order,
                relation
                    .metadata
                    .get("column")
                    .cloned()
                    .unwrap_or_else(|| crate::preview_domain::snake(&relation.name)),
                relation.required,
            )
        }))
        .collect();
    fields.sort_by_key(|item| item.0);
    let attached_properties = entity.metadata_ordered.iter()
        .any(|(name, value)| name == "properties" && !value.is_empty());
    let mut output = format!(
        "\n{{\n  \"count\": 2,\n  \"depth\": {},\n  \"meta\": [\n",
        depth
    );
    if attached_properties {
        output.push_str("{\n      \"type\": \"\",\n      \"required\": \"\",\n      \"name\": \"\",\n      \"localeName\": \"\"\n    }");
    }
    for (index, (_, name, required)) in fields.iter().enumerate().filter(|_| !attached_properties) {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&format!("{{\n      \"type\": \"\",\n      \"required\": \"{required}\",\n      \"name\": \"{name}\",\n      \"localeName\": \"{name}\"\n    }}"));
    }
    output.push_str("\n  ],\n");
    if entity.constant {
        output.push_str("  \"presetData\": [\n");
        for (index, candidate) in entity.candidates.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&format!(
                "{{\"name\":\"{}\",\"code\":\"{}\"}}",
                candidate.rust_candidate_method_value, candidate.rust_candidate_filter_value
            ));
        }
        output.push_str("\n  ],\n\n");
    }
    output.push_str(&format!("  \"name\": \"{}\",\n  \"id\": \"{}\",\n  \"_class\":\"class com.skynet.codegenerator.meta.ObjectDescriptor\",\n  \"_domain_class\":\"class com.skynet.codegenerator.meta.Domain\"\n\n}}\n", entity.name, entity.name));
    output
}

fn links(domain: &RenderDomain) -> String {
    let mut groups = Vec::new();
    for entity in &domain.entities {
        let mut group = Vec::new();
        for relation in &entity.reverse_relations {
            let target = domain
                .entities
                .iter()
                .find(|candidate| candidate.rust_struct == relation.target_struct);
            if let Some(target) = target {
                group.push(format!("{{\n       \"source\": \"{}\",\n       \"type\": \"{}\",\n       \"target\": \"{}\",\n       \"value\": 1,\n       \"refBy\": \"{}\"\n }}", entity.name, target.name, target.name, relation.java_member_name));
            }
        }
        for child in entity
            .children(domain)
            .into_iter()
            .filter(|_| !entity.reverse_relations.is_empty())
        {
            group.push(format!("{{\n       \"source\": \"{}\",\n       \"type\": \"{}\",\n       \"target\": \"{}\",\n       \"value\": 1,\n       \"refBy\": \"{}List\"\n }}", entity.name, child.name, child.name, crate::preview_domain::camel(&child.name)));
        }
        if !group.is_empty() {
            groups.push(group.join(","));
        }
    }
    if groups.is_empty() {
        String::new()
    } else {
        format!("{} ", groups.join(" ,"))
    }
}

pub(crate) fn render_model_view(domain: &RenderDomain) -> String {
    let depths = crate::preview_objects::source_depths(domain);
    let nodes = domain
        .entities
        .iter()
        .zip(depths)
        .map(|(entity, depth)| node(entity, depth))
        .collect::<Vec<_>>()
        .join(",");
    let domain_name = st4_rendered_line_endings(domain.name.clone());
    let indented_name = domain_name.replace('\n', "\n    ");
    st4_rendered_line_endings(include_str!("../templates/doc/model-view-source.txt")
        .replace("    __DOMAIN_NAME__", &format!("    {indented_name}"))
        .replace("__DOMAIN_NAME__", &domain_name)
        .replace("__NODES__", &nodes)
        .replace("__LINKS__", &links(domain)))
}
