//! Shared utilities for cospan-codegen.

use panproto_gat::Name;
use panproto_protocols::emit::children_by_edge;
use panproto_schema::Schema;

/// Convert camelCase to snake_case.
pub fn camel_to_snake(s: &str) -> String {
    let mut r = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            r.push('_');
        }
        r.push(c.to_lowercase().next().unwrap_or(c));
    }
    r
}

/// Convert an NSID like "dev.cospan.repo.issue" to PascalCase "RepoIssue".
pub fn nsid_to_pascal(nsid: &str) -> String {
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

/// Find the record body vertex ID for an NSID.
pub fn find_record_body(schema: &Schema, nsid: &str) -> String {
    let children = children_by_edge(schema, nsid, "record-schema");
    if let Some((_, body)) = children.first() {
        return body.id.to_string();
    }
    nsid.to_string()
}

/// Check if a field is required in a schema record.
pub fn is_field_required(schema: &Schema, body_id: &str, field_name: &str) -> bool {
    schema
        .required
        .get(&Name::from(body_id))
        .map(|reqs| {
            reqs.iter()
                .any(|e| e.name.as_ref().map(|n| n.as_str()) == Some(field_name))
        })
        .unwrap_or(false)
}

/// Convert snake_case to camelCase.
pub fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap_or(c));
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}
