use serde_derive::{Deserialize, Serialize};

pub mod batch;
pub mod transfer;

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
pub struct Object {
    pub oid: String,
    pub size: u64,
}

pub const GIT_LFS_CONTENT_TYPE: &str = "application/vnd.git-lfs+json";
