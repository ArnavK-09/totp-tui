use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Account {
    pub(crate) id: String,
    pub(crate) issuer: String,
    pub(crate) account_name: String,
    pub(crate) period: u32,
    pub(crate) digits: u32,
}
