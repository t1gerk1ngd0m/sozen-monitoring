use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Slot {
  pub date: String,
  pub time: String,
  pub label: Option<String>,
}
