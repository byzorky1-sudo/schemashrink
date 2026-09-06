use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallAccuracy {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub accuracy_pct: f64,
    pub avg_ttft_ms_original: f64,
    pub avg_ttft_ms_compressed: f64,
    pub ttft_speedup_pct: f64,
}

pub struct SchemaValidator;

impl SchemaValidator {
    /// Validates that a compressed schema maintains the required fields and properties of the original
    pub fn validate_schema_integrity(original: &Value, compressed: &Value) -> bool {
        // Check top-level types match
        match (original, compressed) {
            (Value::Object(orig_map), Value::Object(comp_map)) => {
                // If original has "properties", compressed must preserve them
                if let (Some(Value::Object(orig_props)), Some(Value::Object(comp_props))) =
                    (orig_map.get("properties"), comp_map.get("properties"))
                {
                    for (prop_name, orig_val) in orig_props {
                        if let Some(comp_val) = comp_props.get(prop_name) {
                            if !Self::validate_schema_integrity(orig_val, comp_val) {
                                return false;
                            }
                        } else {
                            return false; // Missing property
                        }
                    }
                }

                // If original has "required", compressed must preserve all required keys
                if let (Some(Value::Array(orig_req)), Some(Value::Array(comp_req))) =
                    (orig_map.get("required"), comp_map.get("required"))
                {
                    let orig_set: std::collections::HashSet<_> = orig_req.iter().collect();
                    let comp_set: std::collections::HashSet<_> = comp_req.iter().collect();
                    if orig_set != comp_set {
                        return false;
                    }
                }

                true
            }
            (Value::Array(orig_arr), Value::Array(comp_arr)) => {
                if orig_arr.len() != comp_arr.len() {
                    return false;
                }
                for (o, c) in orig_arr.iter().zip(comp_arr.iter()) {
                    if !Self::validate_schema_integrity(o, c) {
                        return false;
                    }
                }
                true
            }
            _ => true,
        }
    }
}
