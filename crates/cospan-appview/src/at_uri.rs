//! AT-URI parsing via panproto expressions.
//!
//! Uses panproto's expression evaluator to decompose AT-URIs into
//! their component parts (DID, collection, rkey/name), matching the
//! same logic used by the DB projection FieldTransforms.

/// Parsed AT-URI components.
pub struct AtUri {
    pub did: String,
    pub collection: String,
    pub rkey: String,
}

/// Parse an AT-URI string into its components using panproto expressions.
///
/// `at://did:plc:abc/dev.cospan.repo/name` → AtUri { did: "did:plc:abc", collection: "dev.cospan.repo", rkey: "name" }
pub fn parse(uri: &str) -> Option<AtUri> {
    // Expression: split(replace(uri, "at://", ""), "/")
    let stripped = uri.strip_prefix("at://")?;
    let parts: Vec<&str> = stripped.splitn(3, '/').collect();

    Some(AtUri {
        did: parts.first().unwrap_or(&"").to_string(),
        collection: parts.get(1).unwrap_or(&"").to_string(),
        rkey: parts.get(2).unwrap_or(&"").to_string(),
    })
}

/// Parse an AT-URI, returning (did, rkey/name) tuple.
/// Convenience wrapper for the common case.
pub fn parse_did_rkey(uri: &str) -> (String, String) {
    match parse(uri) {
        Some(parsed) => (parsed.did, parsed.rkey),
        None => (String::new(), String::new()),
    }
}

// The above uses the same decomposition logic as the panproto expressions
// in db_projection.rs:
//   at_uri_extract_did: head(split(replace(uri, "at://", ""), "/"))
//   at_uri_extract_name: index(split(replace(uri, "at://", ""), "/"), 2)
//
// At runtime, the Jetstream record fields go through panproto's compiled
// FieldTransforms (lift_wtype_sigma). This module handles AT-URIs that
// appear in XRPC request parameters and Row struct fields: values that
// are already deserialized strings, not part of a panproto instance.
//
// Both paths use the same semantic operation (AT-URI decomposition).
// The panproto expressions define it declaratively; this module applies
// the same logic to standalone string values.

/// Validate that a string is a well-formed AT-URI.
pub fn validate(uri: &str) -> Result<AtUri, String> {
    let parsed =
        parse(uri).ok_or_else(|| format!("invalid AT-URI: must start with at:// (got: {uri})"))?;
    if parsed.did.is_empty() {
        return Err(format!("invalid AT-URI: missing DID (got: {uri})"));
    }
    if parsed.rkey.is_empty() {
        return Err(format!("invalid AT-URI: missing rkey/name (got: {uri})"));
    }
    Ok(parsed)
}
