use crate::context::{RenderDomain, RenderEntity};
use std::fmt::Write;

fn skip(source_type: &str, name: &str) -> bool {
    matches!(source_type, "id" | "version" | "createTime" | "updateTime")
        || matches!(name, "id" | "version")
}

fn constant_fields(
    entity: &RenderEntity,
    seed: &teaql_forge_model::ir::SeedValue,
) -> Result<Vec<(String, String, bool)>, String> {
    let mut fields = Vec::new();
    for field in &entity.fields {
        if skip(&field.source_type_name, &field.name) {
            continue;
        }
        let index = entity
            .seed_values
            .iter()
            .position(|row| std::ptr::eq(row, seed))
            .unwrap();
        let value = crate::preview_rust::bootstrap_literal(entity, field, Some(index))?;
        fields.push((
            field.source_order,
            field.rust_storage_member_name.clone(),
            value,
            !field.required,
        ));
    }
    for relation in &entity.object_fields {
        fields.push((
            relation.source_order,
            relation.rust_storage_member_name.clone(),
            "1_u64".into(),
            !relation.required,
        ));
    }
    fields.sort_by_key(|(order, _, _, _)| *order);
    Ok(fields
        .into_iter()
        .map(|(_, name, value, optional)| (name, value, optional))
        .collect())
}

pub(crate) fn render(domain: &RenderDomain) -> Result<String, String> {
    let roots = &domain.root_descriptors;
    if roots.len() != 1 {
        return Err(format!(
            "Rust bootstrap requires exactly one Domain Root, found {}",
            roots.len()
        ));
    }
    let root = &roots[0];
    let mut out =
        include_str!("../templates/rust-lib-core/runtime/bootstrap-header.txt").to_string();
    writeln!(out, "        let root_rows = crate::Q::{}().select_self_fields().with_id_is(1_u64).comment(\"what: locate generated Domain Root\").purpose(\"why: idempotent runtime bootstrap\").execute_for_list(context).await.map_err(|e| teaql_runtime::RuntimeError::Graph(e.to_string()))?;", root.rust_plural).unwrap();
    out.push_str("        let domain_root = if let Some(entity) = root_rows.data.into_iter().next() { entity } else {\n");
    writeln!(
        out,
        "            let mut entity = {}::runtime_new(context.entity_runtime_state());",
        root.rust_struct
    )
    .unwrap();
    out.push_str("            entity.update_id(1_u64);\n");
    writeln!(out, "            context.initialize_generated_bootstrap_entity(&mut entity, {}::ENTITY_NAME, 1_u64)?;", root.rust_struct).unwrap();
    for field in &root.fields {
        if skip(&field.source_type_name, &field.name) {
            continue;
        }
        let value = crate::preview_rust::bootstrap_literal(root, field, None)?;
        writeln!(
            out,
            "            entity.update_{}({value});",
            field.rust_storage_member_name
        )
        .unwrap();
    }
    writeln!(out, "            teaql_runtime::AuditedSaveExt::save(entity.audit_as(\"create generated Domain Root {}\"), context).await?", root.rust_struct).unwrap();
    out.push_str("        };\n");
    writeln!(
        out,
        "        context.set_generated_bootstrap_active_root({}::ENTITY_NAME, domain_root.id())?;",
        root.rust_struct
    )
    .unwrap();
    for entity in &domain.constant_descriptors {
        for seed in &entity.seed_values {
            let Some(id) = seed
                .properties
                .get("id")
                .and_then(|raw| raw.parse::<u64>().ok())
            else {
                continue;
            };
            let variable = format!("constant_{}_{}", entity.rust_module, id);
            let rows = format!("rows_{variable}");
            writeln!(out, "        let {rows} = crate::Q::{}().select_self_fields().with_id_is({id}_u64).comment(\"what: locate generated constant\").purpose(\"why: idempotent runtime bootstrap\").execute_for_list(context).await.map_err(|e| teaql_runtime::RuntimeError::Graph(e.to_string()))?;", entity.rust_plural).unwrap();
            writeln!(
                out,
                "        if let Some(mut {variable}) = {rows}.data.into_iter().next() {{"
            )
            .unwrap();
            out.push_str("            let mut changed = false;\n");
            let fields = constant_fields(entity, seed)?;
            for (name, value, optional) in &fields {
                let expected = if *optional {
                    format!("Some(({value}).into())")
                } else {
                    value.clone()
                };
                writeln!(out, "            if {variable}.{name}() != {expected} {{ {variable}.update_{name}({value}); changed = true; }}").unwrap();
            }
            writeln!(out, "            if changed {{ let _ = teaql_runtime::AuditedSaveExt::save({variable}.audit_as(\"reconcile model constant {}({id})\"), context).await?; }}", entity.rust_struct).unwrap();
            out.push_str("        } else {\n");
            writeln!(
                out,
                "            let mut {variable} = {}::runtime_new(context.entity_runtime_state());",
                entity.rust_struct
            )
            .unwrap();
            writeln!(out, "            {variable}.update_id({id}_u64);").unwrap();
            writeln!(out, "            context.initialize_generated_bootstrap_entity(&mut {variable}, {}::ENTITY_NAME, {id}_u64)?;", entity.rust_struct).unwrap();
            for (name, value, _) in &fields {
                writeln!(out, "            {variable}.update_{name}({value});").unwrap();
            }
            writeln!(out, "            let _ = teaql_runtime::AuditedSaveExt::save({variable}.audit_as(\"create model constant {}({id})\"), context).await?;", entity.rust_struct).unwrap();
            out.push_str("        }\n");
        }
    }
    out.push_str("        Ok(())\n}\n");
    Ok(out)
}
