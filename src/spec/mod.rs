pub mod batch;
pub mod transfer;

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Object {
    pub oid: String,
    pub size: u64,
}
