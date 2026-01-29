use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::map::Cell;

#[derive(Debug, Deserialize, Serialize)]
pub struct GameMessage {
    pub msg: String,
    pub cells: Option<Vec<Cell>>,
    #[serde(flatten)]
    pub other: serde_json::Map<String, Value>,
}

pub fn normalize_messages(value: Value) -> Vec<Value> {
    if let Some(arr) = value.as_array() {
        arr.clone()
    } else if let Some(msgs_arr) = value.get("msgs").and_then(|m| m.as_array()) {
        msgs_arr.clone()
    } else {
        vec![value]
    }
}
