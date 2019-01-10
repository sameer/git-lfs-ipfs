use serde_derive::Deserialize;

use crate::spec::Object;

/// See https://github.com/git-lfs/git-lfs/blob/master/docs/api/basic-transfers.md
#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct VerifyRequest {
    #[serde(flatten)]
    pub object: Object,
}
