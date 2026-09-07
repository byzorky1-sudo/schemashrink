use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Strip verbose boilerplate fields like `$schema`, `additionalProperties: false`, `title` when redundant
    pub strip_meta: bool,
    /// Sort object keys deterministically to guarantee byte-level prefix KV-cache hit rate
    pub deterministic_sort: bool,
    /// Truncate long descriptions while preserving essential typing constraints
    pub prune_descriptions: bool,
    /// Strip type descriptions entirely (aggressive compression mode)
    pub strip_descriptions: bool,
    /// Maximum allowed description length in characters (if prune_descriptions is true)
    pub max_desc_len: usize,
    /// Minify JSON output (remove whitespace)
    pub minify: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            strip_meta: true,
            deterministic_sort: true,
            prune_descriptions: false,
            strip_descriptions: false,
            max_desc_len: 120,
            minify: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    pub original_chars: usize,
    pub compressed_chars: usize,
    pub original_tokens_est: usize,
    pub compressed_tokens_est: usize,
    pub token_savings_pct: f64,
    pub schema_hash: String,
    pub compressed_json: String,
}

pub struct SchemaCompressor {
    config: CompressionConfig,
}

impl SchemaCompressor {
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    pub fn with_default() -> Self {
        Self::new(CompressionConfig::default())
    }

    /// Compresses a raw JSON string of tool definitions or schemas
    pub fn compress_str(&self, raw_json: &str) -> Result<CompressionResult, serde_json::Error> {
        let val: Value = serde_json::from_str(raw_json)?;
        let compressed_val = self.compress_value(&val);

        let original_chars = raw_json.len();
        let compressed_json = if self.config.minify {
            serde_json::to_string(&compressed_val)?
        } else {
            serde_json::to_string_pretty(&compressed_val)?
        };
        let compressed_chars = compressed_json.len();

        let original_tokens_est = original_chars.div_ceil(4);
        let compressed_tokens_est = compressed_chars.div_ceil(4);
        let token_savings_pct = if original_tokens_est > 0 {
            ((original_tokens_est as f64 - compressed_tokens_est as f64)
                / original_tokens_est as f64)
                * 100.0
        } else {
            0.0
        };

        // Compute deterministic SHA-256 hash for prefix cache tracking
        let mut hasher = Sha256::new();
        hasher.update(compressed_json.as_bytes());
        let schema_hash = hex::encode(hasher.finalize());

        Ok(CompressionResult {
            original_chars,
            compressed_chars,
            original_tokens_est,
            compressed_tokens_est,
            token_savings_pct,
            schema_hash,
            compressed_json,
        })
    }

    /// Recursively compresses a serde_json Value
    pub fn compress_value(&self, value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut new_map = Map::new();

                // Sort keys deterministically if enabled
                let mut keys: Vec<&String> = map.keys().collect();
                if self.config.deterministic_sort {
                    keys.sort();
                }

                for k in keys {
                    let v = &map[k];

                    // Strip metadata fields if configured
                    if self.config.strip_meta {
                        if k == "$schema" || k == "title" {
                            continue;
                        }
                        if k == "additionalProperties" && v == &Value::Bool(false) {
                            continue;
                        }
                    }

                    // Strip or prune descriptions
                    if k == "description" {
                        if self.config.strip_descriptions {
                            continue;
                        }
                        if self.config.prune_descriptions {
                            if let Value::String(s) = v {
                                if s.len() > self.config.max_desc_len {
                                    let truncated =
                                        format!("{}...", &s[..self.config.max_desc_len - 3]);
                                    new_map.insert(k.clone(), Value::String(truncated));
                                    continue;
                                }
                            }
                        }
                    }

                    // Recursively compress nested fields
                    new_map.insert(k.clone(), self.compress_value(v));
                }

                Value::Object(new_map)
            }
            Value::Array(arr) => {
                let new_arr = arr.iter().map(|item| self.compress_value(item)).collect();
                Value::Array(new_arr)
            }
            _ => value.clone(),
        }
    }
}
