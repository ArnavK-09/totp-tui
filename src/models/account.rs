use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub issuer: String,
    pub account_name: String,
    pub period: u32,
    pub digits: u32,
}
