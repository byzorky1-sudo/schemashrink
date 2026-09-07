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

#[test]
fn test_strip_descriptions_mode() {
    let raw_schema = json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "A very long detailed parameter description that takes many tokens"
            }
        },
        "required": ["query"]
    });

    let config = CompressionConfig {
        strip_descriptions: true,
        ..Default::default()
    };
    let compressor = SchemaCompressor::new(config);
    let res = compressor
        .compress_str(&serde_json::to_string(&raw_schema).unwrap())
        .unwrap();

    let comp_val: serde_json::Value = serde_json::from_str(&res.compressed_json).unwrap();
    assert!(comp_val["properties"]["query"].get("description").is_none());
    assert_eq!(comp_val["properties"]["query"]["type"], "string");
}

#[test]
fn test_prune_descriptions_mode() {
    let raw_schema = json!({
        "type": "object",
        "properties": {
            "param": {
                "type": "string",
                "description": "123456789012345678901234567890"
            }
        }
    });

    let config = CompressionConfig {
        prune_descriptions: true,
        max_desc_len: 15,
        ..Default::default()
    };
    let compressor = SchemaCompressor::new(config);
    let res = compressor
        .compress_str(&serde_json::to_string(&raw_schema).unwrap())
        .unwrap();

    let comp_val: serde_json::Value = serde_json::from_str(&res.compressed_json).unwrap();
    let desc = comp_val["properties"]["param"]["description"]
        .as_str()
        .unwrap();
    assert_eq!(desc, "123456789012...");
    assert_eq!(desc.len(), 15);
}

#[test]
fn test_openai_anthropic_hermes_tool_payloads() {
    let hermes_tools = json!([
        {
            "type": "function",
            "function": {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "read_file",
                "name": "read_file",
                "description": "Read contents of a file from disk with line numbering.",
                "parameters": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute path to file."
                        },
                        "offset": {
                            "type": "integer",
                            "description": "1-based line offset."
                        }
                    },
                    "required": ["path"]
                }
            }
        }
    ]);

    let compressor = SchemaCompressor::with_default();
    let res = compressor
        .compress_str(&serde_json::to_string_pretty(&hermes_tools).unwrap())
        .unwrap();

    assert!(res.token_savings_pct > 30.0);
    let comp_val: serde_json::Value = serde_json::from_str(&res.compressed_json).unwrap();

    // Verify properties and requirements preserved
    let fn_obj = &comp_val[0]["function"];
    assert_eq!(fn_obj["name"], "read_file");
    assert!(fn_obj.get("$schema").is_none());
    assert!(fn_obj.get("title").is_none());
    assert_eq!(fn_obj["parameters"]["properties"]["path"]["type"], "string");
    assert_eq!(fn_obj["parameters"]["required"][0], "path");
}
