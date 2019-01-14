use actix_web::http::StatusCode;
use chrono::{DateTime, FixedOffset};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::spec::Object;

#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct BatchRequest {
    pub operation: Operation,
    #[serde(default = "Transfer::default_vec")]
    pub transfer: Vec<Transfer>,
    #[serde(rename = "ref")]
    pub ref_property: Option<Ref>,
    pub objects: Vec<Object>,
}

#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct BatchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer: Option<Transfer>,
    pub objects: Vec<ObjectResponse>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Download,
    Upload,
}

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

#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct Ref {
    pub name: String,
}

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
            actions: actions,
        }
    }

    pub fn error(object: Object, error: ObjectError) -> Self {
        ObjectResponse::Error { object, error }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct ObjectSuccess {
    #[serde(skip_serializing_if = "Option::is_none")]
    authenticated: Option<bool>,
    actions: Actions,
}

#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct ObjectError {
    code: u16,
    message: &'static str,
}

impl ObjectError {
    pub fn DoesNotExist() -> Self {
        Self {
            code: StatusCode::NOT_FOUND.as_u16(),
            message: "Object does not exist",
        }
    }

    pub fn RemovedByOwner() -> Self {
        Self {
            code: StatusCode::GONE.as_u16(),
            message: "Object removed by owner",
        }
    }
    pub fn ValidationError() -> Self {
        Self {
            code: StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
            message: "Validation error",
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct Actions {
    #[serde(skip_serializing_if = "Option::is_none")]
    download: Option<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upload: Option<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: Option<Action>,
}

impl Actions {
    pub fn download(download: Action) -> Self {
        Self {
            download: Some(download),
            upload: None,
            verify: None,
        }
    }

    pub fn upload_and_verify(upload: Action, verify: Action) -> Self {
        Self {
            download: None,
            upload: Some(upload),
            verify: Some(verify),
        }
    }

    pub fn upload(upload: Action) -> Self {
        Self {
            download: None,
            upload: Some(upload),
            verify: None,
        }
    }
}

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

#[derive(PartialEq, Eq, Debug, Serialize)]
pub struct LfsErrorResponse {
    message: &'static str,
    #[serde(with = "url_serde")]
    documentation_url: Option<Url>,
    request_id: Option<String>,
    #[serde(skip)]
    status: StatusCode,
}

impl LfsErrorResponse {
    const ACCEPT_HEADER_INCORRECT: Self = Self {
        message: "The Accept header needs to be `application/vnd.git-lfs+json`.",
        documentation_url: None,
        request_id: None,
        status: StatusCode::NOT_ACCEPTABLE,
    };
    const RATE_LIMIT_HIT: Self = Self {
        message: "A rate limit has been hit with the server.",
        documentation_url: None,
        request_id: None,
        status: StatusCode::TOO_MANY_REQUESTS,
    };
    const NOT_IMPLEMENTED: Self = Self {
        message: "The server has not implemented the current method.",
        documentation_url: None,
        request_id: None,
        status: StatusCode::NOT_IMPLEMENTED,
    };
    const INSUFFICIENT_STORAGE: Self = Self {
        message: "The server has insufficient storage capacity to complete the request.",
        documentation_url: None,
        request_id: None,
        status: StatusCode::INSUFFICIENT_STORAGE,
    };
    // const BANDWIDTH_LIMIT_EXCEEDED: Self = Self {
    //     message: "A bandwidth limit has been exceeded.",
    //     documentation_url: None,
    //     request_id: None,
    //     status: StatusCode::from_u16(509).unwrap(),
    // };
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
                    actions: Actions {
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
                        .into(),
                        upload: None,
                        verify: None,
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
                status: StatusCode::NOT_FOUND
            })
            .unwrap(),
        );
    }

}
