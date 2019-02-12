use chrono::{DateTime, FixedOffset};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::spec::Object;

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/batch.md#requests
#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct BatchRequest {
    pub operation: Operation,
    #[serde(default = "Transfer::default_vec")]
    pub transfer: Vec<Transfer>,
    #[serde(rename = "ref")]
    pub ref_property: Option<Ref>,
    pub objects: Vec<Object>,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/batch.md#successful-responses
#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct BatchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer: Option<Transfer>,
    pub objects: Vec<ObjectResponse>,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/batch.md#requests
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Download,
    Upload,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/basic-transfers.md#basic-transfer-api
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Transfer {
    Basic,
    Custom,
}

impl Transfer {
    fn default_vec() -> Vec<Self> {
        vec![Transfer::Basic]
    }
}

impl Default for Transfer {
    fn default() -> Self {
        Transfer::Basic
    }
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/batch.md#ref-property
#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct Ref {
    pub name: String,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/batch.md#successful-responses
#[derive(PartialEq, Eq, Debug, Serialize)]
#[serde(untagged)]
pub enum ObjectResponse {
    Success {
        #[serde(flatten)]
        object: Object,
        #[serde(skip_serializing_if = "Option::is_none")]
        authenticated: Option<bool>,
        actions: Actions,
    },
    Error {
        #[serde(flatten)]
        object: Object,
        error: ObjectError,
    },
}

impl ObjectResponse {
    pub fn success(object: Object, actions: Actions) -> Self {
        ObjectResponse::Success {
            object,
            authenticated: None,
            actions,
        }
    }

    pub fn error(object: Object, error: ObjectError) -> Self {
        ObjectResponse::Error { object, error }
    }
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/batch.md#successful-responses
#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct ObjectSuccess {
    #[serde(skip_serializing_if = "Option::is_none")]
    authenticated: Option<bool>,
    actions: Actions,
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/batch.md#response-errors
#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct ObjectError {
    code: u16,
    message: &'static str,
}

impl ObjectError {
    pub fn DoesNotExist() -> Self {
        Self {
            code: 404u16,
            message: "Object does not exist",
        }
    }

    pub fn RemovedByOwner() -> Self {
        Self {
            code: 410u16,
            message: "Object removed by owner",
        }
    }
    pub fn ValidationError() -> Self {
        Self {
            code: 422u16,
            message: "Validation error",
        }
    }
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/basic-transfers.md#basic-transfer-api
#[derive(PartialEq, Eq, Debug, Serialize)]
#[serde(untagged)]
pub enum Actions {
    Download { download: Action },
    None,
    Upload { upload: Action },
    UploadAndVerify { upload: Action, verify: Action },
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/basic-transfers.md#basic-transfer-api
#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct Action {
    #[serde(with = "url_serde")]
    href: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    header: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<DateTime<FixedOffset>>,
}

impl Action {
    pub fn new(href: Url) -> Self {
        Self {
            href,
            header: None,
            expires_in: None,
            expires_at: None,
        }
    }
}

/// https://github.com/git-lfs/git-lfs/blob/master/docs/api/batch.md#response-errors
#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct LfsErrorResponse {
    message: &'static str,
    #[serde(with = "url_serde")]
    documentation_url: Option<Url>,
    request_id: Option<String>,
    #[serde(skip)]
    status: u16,
}

impl LfsErrorResponse {
    const ACCEPT_HEADER_INCORRECT: Self = Self {
        message: "The Accept header needs to be `application/vnd.git-lfs+json`.",
        documentation_url: None,
        request_id: None,
        status: 406u16,
    };
    const RATE_LIMIT_HIT: Self = Self {
        message: "A rate limit has been hit with the server.",
        documentation_url: None,
        request_id: None,
        status: 429u16,
    };
    const NOT_IMPLEMENTED: Self = Self {
        message: "The server has not implemented the current method.",
        documentation_url: None,
        request_id: None,
        status: 501u16,
    };
    const INSUFFICIENT_STORAGE: Self = Self {
        message: "The server has insufficient storage capacity to complete the request.",
        documentation_url: None,
        request_id: None,
        status: 507u16,
    };

    const BANDWIDTH_LIMIT_EXCEEDED: Self = Self {
        message: "A bandwidth limit has been exceeded.",
        documentation_url: None,
        request_id: None,
        status: 509u16,
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn batch_response_serializes_correctly() {
        assert_eq!(
            include_str!("test/batch_response_success.json"),
            serde_json::to_string_pretty(&BatchResponse {
                transfer: Some(Transfer::Basic),
                objects: vec![ObjectResponse::Success {
                    object: Object {
                        oid: "1111111".to_string(),
                        size: 123,
                    },
                    authenticated: Some(true),
                    actions: Actions::Download {
                        download: Action {
                            href: Url::parse("https://some-download.com").unwrap(),
                            header: Some(
                                [("Key", "value")]
                                    .iter()
                                    .map(|(k, v)| (k.to_string(), v.to_string()))
                                    .collect()
                            ),
                            expires_in: None,
                            expires_at: DateTime::parse_from_rfc3339("2016-11-10T15:29:07Z")
                                .unwrap()
                                .into()
                        }
                    }
                }],
            })
            .unwrap(),
        );

        assert_eq!(
            include_str!("test/batch_response_error.json"),
            serde_json::to_string_pretty(&BatchResponse {
                transfer: Some(Transfer::Basic),
                objects: vec![ObjectResponse::Error {
                    error: ObjectError::DoesNotExist(),
                    object: Object {
                        oid: "1111111".to_string(),
                        size: 123,
                    },
                }],
            })
            .unwrap()
        );
    }

    #[test]
    fn lfs_error_serializes_correctly() {
        assert_eq!(
            include_str!("test/lfs_error.json"),
            serde_json::to_string_pretty(&LfsErrorResponse {
                message: "Not found",
                documentation_url: Url::parse("https://lfs-server.com/docs/errors")
                    .unwrap()
                    .into(),
                request_id: Some("123".to_string()),
                status: 404u16
            })
            .unwrap(),
        );
    }

}
