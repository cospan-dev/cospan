//! Generate Rust Input/Output/Params types from Lexicon query/procedure definitions.
//!
//! Parses query parameters and procedure input schemas via panproto's
//! ATProto protocol parser, then emits Rust structs with serde derives.

use anyhow::Result;
use panproto_protocols::emit::{IndentWriter, children_by_edge, constraint_value};
use panproto_schema::Schema;

/// Emit Rust types for a query (Params struct) or procedure (Input struct).
pub fn emit_xrpc_types(schema: &Schema, nsid: &str, def_type: &str) -> Result<String> {
    let mut w = IndentWriter::new("    ");

    match def_type {
        "query" => {
            let params_vertex = format!("{nsid}:params");
            if schema.has_vertex(&params_vertex) {
                let struct_name = format!("{}Params", nsid_to_pascal(nsid));
                emit_struct(&mut w, schema, &params_vertex, &struct_name, "Deserialize");
            }
        }
        "procedure" => {
            let input_vertex = format!("{nsid}:input");
            if schema.has_vertex(&input_vertex) {
                let struct_name = format!("{}Input", nsid_to_pascal(nsid));
                emit_struct(&mut w, schema, &input_vertex, &struct_name, "Deserialize");
            }
        }
        "subscription" => {
            // Subscriptions don't have input types to generate
        }
        _ => {}
    }

    Ok(w.finish())
}

fn emit_struct(
    w: &mut IndentWriter,
    schema: &Schema,
    vertex_id: &str,
    struct_name: &str,
    derives: &str,
) {
    let props = children_by_edge(schema, vertex_id, "prop");
    if props.is_empty() {
        return;
    }

    w.line(&format!("#[derive(Debug, serde::{derives})]"));
    w.line("#[serde(rename_all = \"camelCase\")]");
    w.line(&format!("pub struct {struct_name} {{"));
    w.indent();

    for (edge, prop_vertex) in &props {
        let field_name = edge.name.as_ref().map(|n| n.as_str()).unwrap_or("unknown");
        let snake = camel_to_snake(field_name);
        let is_required = is_field_required(schema, vertex_id, field_name);
        let rust_type = kind_to_rust_type(&prop_vertex.kind, field_name);

        let full_type = if is_required {
            rust_type
        } else {
            format!("Option<{rust_type}>")
        };

        w.line(&format!("pub {snake}: {full_type},"));
    }

    w.dedent();
    w.line("}");
    w.blank();
}

fn kind_to_rust_type(kind: &panproto_gat::Name, _field_name: &str) -> String {
    match kind.as_str() {
        "string" => "String".into(),
        "integer" => "i64".into(),
        "number" => "f64".into(),
        "boolean" => "bool".into(),
        _ => "String".into(),
    }
}

fn is_field_required(schema: &Schema, vertex_id: &str, field_name: &str) -> bool {
    use panproto_gat::Name;
    schema
        .required
        .get(&Name::from(vertex_id))
        .map(|reqs| {
            reqs.iter()
                .any(|e| e.name.as_ref().map(|n| n.as_str()) == Some(field_name))
        })
        .unwrap_or(false)
}

fn nsid_to_pascal(nsid: &str) -> String {
    nsid.split('.')
        .skip(2)
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                None => String::new(),
                Some(ch) => ch.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect()
}

fn camel_to_snake(s: &str) -> String {
    let mut r = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            r.push('_');
        }
        r.push(c.to_lowercase().next().unwrap_or(c));
    }
    r
}
