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


#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LsResponse {
    pub objects: Vec<Object>
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Object {
    pub hash: String,
    pub links: Vec<Link>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectResponse {
    #[serde(flatten)]
    pub object: Object
}


#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Link {
    pub name: String,
    pub hash: String,
    pub size: String,
    pub Type: String, // Not sure how to handle this
}
