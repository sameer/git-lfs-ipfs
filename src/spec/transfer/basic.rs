use serde_derive::Deserialize;

use crate::spec::Object;

/// See https://github.com/git-lfs/git-lfs/blob/master/docs/api/basic-transfers.md
#[derive(PartialEq, Eq, Debug, Deserialize)]
#[serde(transparent)]
pub struct VerifyRequest {
    pub object: Object,
}
