use serde_derive::{Deserialize};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddResponse {
    pub name: String,
    pub hash: String,
    pub size: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KeyListResponse {
    pub keys: Vec<Key>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Key {
    pub name: String,
    pub id: String,
}
