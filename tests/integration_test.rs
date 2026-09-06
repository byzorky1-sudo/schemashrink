use schemashrink::{
    compressor::{CompressionConfig, SchemaCompressor},
    validator::SchemaValidator,
};
use serde_json::json;

#[test]
fn test_lossless_schema_compression() {
    let raw_schema = r#"{
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "SearchTool",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "query": {
                "type": "string",
                "description": "The search query to look up on the web"
            },
            "limit": {
                "type": "integer",
                "description": "Maximum results to return"
            }
        },
        "required": ["query"]
    }"#;

    let compressor = SchemaCompressor::with_default();
    let res = compressor.compress_str(raw_schema).unwrap();

    assert!(res.compressed_chars < res.original_chars);
    assert!(res.token_savings_pct > 30.0);

    // Verify structural integrity
    let orig_val: serde_json::Value = serde_json::from_str(raw_schema).unwrap();
    let comp_val: serde_json::Value = serde_json::from_str(&res.compressed_json).unwrap();
    assert!(SchemaValidator::validate_schema_integrity(
        &orig_val, &comp_val
    ));
}

#[test]
fn test_deterministic_hashing() {
    let compressor = SchemaCompressor::with_default();

    // Two schemas with shuffled keys
    let schema_a =
        r#"{"properties": {"b": {"type": "string"}, "a": {"type": "number"}}, "type": "object"}"#;
    let schema_b =
        r#"{"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "string"}}}"#;

    let res_a = compressor.compress_str(schema_a).unwrap();
    let res_b = compressor.compress_str(schema_b).unwrap();

    // Hashing must be 100% identical to guarantee KV-cache reuse!
    assert_eq!(res_a.schema_hash, res_b.schema_hash);
}
