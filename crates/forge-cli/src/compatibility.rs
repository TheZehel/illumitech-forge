//! Public, additive envelopes. Native RPC frames keep their native protocol unchanged.
use serde_json::{Value, json};

pub fn contracts() -> Value {
    serde_json::from_str(include_str!("contracts-v1.json")).expect("checked-in contract")
}
pub fn envelope(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.entry("schema_version").or_insert(json!(1));
        object.insert("tool_version".into(), json!(env!("CARGO_PKG_VERSION")));
    }
    value
}
