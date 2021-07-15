use crate::spec::Object;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

/// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#stage-1-intiation
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    Upload,
    Download,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#stage-1-intiation
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Init {
    pub operation: Operation,
    pub remote: String,
    pub concurrent: bool,
    pub concurrenttransfers: Option<usize>,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#uploads
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Upload {
    #[serde(flatten)]
    pub object: Object,
    pub path: std::path::PathBuf,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#downloads
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    #[serde(flatten)]
    pub object: Object,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#stage-2-0n-transfers
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Complete {
    pub oid: String,
    #[serde(flatten)]
    pub result: Option<Result>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Result {
    Path(PathBuf),
    Error(Error),
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#stage-2-0n-transfers
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: i32,
    pub message: String,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#progress
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub oid: String,
    pub bytes_so_far: u64,
    pub bytes_since_last: u64,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#protocol
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum Event {
    Init(Init),
    #[serde(skip)]
    AcknowledgeInit,
    Upload(Box<Upload>),
    Download(Box<Download>),
    Complete(Box<Complete>),
    Progress(Box<Progress>),
    /// https://github.com/git-lfs/git-lfs/blob/master/docs/custom-transfers.md#stage-3-finish--cleanup
    Terminate,
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    #[test]
    fn custom_complete_serializes_ok() {
        assert_eq!(
            include_str!("../test/custom_complete.json"),
            serde_json::to_string(&Event::Complete(
                Complete {
                    oid: "bf3e3e2af9366a3b704ae0c31de5afa64193ebabffde2091936ad2e7510bc03a"
                        .to_string(),
                    result: Some(Result::Error(Error {
                        code: 2,
                        message: "Explain what happened to this transfer".to_string()
                    })),
                }
                .into()
            ))
            .unwrap(),
        );
    }
    #[test]
    fn custom_download_serializes_ok() {
        assert_eq!(
            include_str!("../test/custom_download.json"),
            serde_json::to_string(&Event::Download(
                Download {
                    object: Object {
                        oid: "22ab5f63670800cc7be06dbed816012b0dc411e774754c7579467d2536a9cf3e"
                            .to_string(),
                        size: 21245,
                    }
                }
                .into()
            ))
            .unwrap(),
        );
    }

    #[test]
    fn custom_init_serializes_ok() {
        assert_eq!(
            include_str!("../test/custom_init.json"),
            serde_json::to_string(&Event::Init(Init {
                operation: Operation::Download,
                remote: "origin".to_string(),
                concurrent: true,
                concurrenttransfers: Some(3)
            }))
            .unwrap(),
        );
    }

    #[test]
    fn custom_progress_serializes_ok() {
        assert_eq!(
            include_str!("../test/custom_progress.json"),
            serde_json::to_string(&Event::Progress(
                Progress {
                    oid: "22ab5f63670800cc7be06dbed816012b0dc411e774754c7579467d2536a9cf3e"
                        .to_string(),
                    bytes_so_far: 1234,
                    bytes_since_last: 64,
                }
                .into()
            ))
            .unwrap(),
        );
    }

    #[test]
    fn custom_terminate_serializes_ok() {
        assert_eq!(
            include_str!("../test/custom_terminate.json"),
            serde_json::to_string(&Event::Terminate).unwrap(),
        );
    }

    #[test]
    fn custom_upload_serializes_ok() {
        assert_eq!(
            include_str!("../test/custom_upload.json"),
            serde_json::to_string(&Event::Upload(
                Upload {
                    object: Object {
                        oid: "bf3e3e2af9366a3b704ae0c31de5afa64193ebabffde2091936ad2e7510bc03a"
                            .to_string(),
                        size: 346232
                    },
                    path: std::path::PathBuf::from_str("/path/to/file.png").unwrap()
                }
                .into()
            ))
            .unwrap(),
        );
    }
}
