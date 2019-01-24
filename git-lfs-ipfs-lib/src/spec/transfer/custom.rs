use serde_derive::{Deserialize, Serialize};

use crate::spec::ipfs::{string, Path};
use crate::spec::Object;

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Init,
    Upload,
    Download,
    Complete,
    Progress,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Upload,
    Download,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum Event {
    Init {
        operation: Operation,
        #[serde(with = "string")]
        remote: Path,
        concurrent: bool,
        concurrenttransfers: Option<usize>,
    },
    Upload {
        #[serde(flatten)]
        object: Object,
        path: std::path::PathBuf,
    },
    Download {
        #[serde(flatten)]
        object: Object,
    },
    Complete {
        oid: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<Error>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<std::path::PathBuf>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        oid: String,
        bytes_so_far: u64,
        bytes_since_last: u64,
    },
    Terminate {},
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Error {
    code: i32,
    message: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn custom_complete_serializes_correctly() {
        assert_eq!(
            include_str!("../test/custom_complete.json"),
            serde_json::to_string(&Event::Complete {
                oid: "bf3e3e2af9366a3b704ae0c31de5afa64193ebabffde2091936ad2e7510bc03a".to_string(),
                error: Some(Error {
                    code: 2,
                    message: "Explain what happened to this transfer".to_string()
                }),
                path: None,
            })
            .unwrap(),
        );
    }
    #[test]
    fn custom_download_serializes_correctly() {
        assert_eq!(
            include_str!("../test/custom_download.json"),
            serde_json::to_string(&Event::Download {
                object: Object {
                    oid: "22ab5f63670800cc7be06dbed816012b0dc411e774754c7579467d2536a9cf3e"
                        .to_string(),
                    size: 21245,
                }
            })
            .unwrap(),
        );
    }

    #[test]
    fn custom_init_serializes_correctly() {
        assert_eq!(
            include_str!("../test/custom_init.json"),
            serde_json::to_string(&Event::Init {
                operation: Operation::Download,
                remote: Path::from_str("/ipfs/QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn")
                    .unwrap(),
                concurrent: true,
                concurrenttransfers: Some(3)
            })
            .unwrap(),
        );
    }

    #[test]
    fn custom_progress_serializes_correctly() {
        assert_eq!(
            include_str!("../test/custom_progress.json"),
            serde_json::to_string(&Event::Progress {
                oid: "22ab5f63670800cc7be06dbed816012b0dc411e774754c7579467d2536a9cf3e".to_string(),
                bytes_so_far: 1234,
                bytes_since_last: 64,
            })
            .unwrap(),
        );
    }

    #[test]
    fn custom_terminate_serializes_correctly() {
        assert_eq!(
            include_str!("../test/custom_terminate.json"),
            serde_json::to_string(&Event::Terminate {}).unwrap(),
        );
    }

    #[test]
    fn custom_upload_serializes_correctly() {
        assert_eq!(
            include_str!("../test/custom_upload.json"),
            serde_json::to_string(&Event::Upload {
                object: Object {
                    oid: "bf3e3e2af9366a3b704ae0c31de5afa64193ebabffde2091936ad2e7510bc03a"
                        .to_string(),
                    size: 346232
                },
                path: std::path::PathBuf::from_str("/path/to/file.png").unwrap()
            })
            .unwrap(),
        );
    }
}
