use chrono::{DateTime, Local};
use rocket::http::Status;
use url::Url;

use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct BatchRequest {
    operation: Operation,
    #[serde(default = vec![Transfer::Basic])]
    transfer: Vec<Transfer>,
    #[serde(rename = "ref")]
    _ref: Option<Ref>,
    objects: Vec<Object>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct BatchResponse {
    transfer: Option<Transfer>,
    objects: Vec<ObjectResponse>,
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

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Ref {
    name: String,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Object {
    oid: String,
    size: u64,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ObjectResponse {
    #[serde(flatten)]
    object: Object,
    error: Option<ObjectError>,
    #[serde(flatten)]
    response: Option<ObjectSuccess>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ObjectSuccess {
    authenticated: Option<bool>,
    #[serde(flatten)]
    action: Actions,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ObjectError {
    code: ObjectErrorCode,
    messange: String,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
enum ObjectErrorCode {
    DoesNotExist = Status::NotFound.code as isize,
    RemovedByOwner = Status::Gone.code as isize,
    ValidationEror = Status::UnprocessableEntity.code as isize,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Actions {
    download: Option<Download>,
    #[serde(flatten)]
    upload: Option<UploadAction>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct UploadAction {
    upload: Upload,
    verify: Option<Verify>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Download {
    #[serde(with = "url_serde")]
    href: Url,
    header: HashMap<String, String>,
    expires_in: Option<i32>,
    expires_at: Option<DateTime<Local>>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Upload {}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Verify {}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct LfsError {
    message: String,
    #[serde(with = "url_serde")]
    documentation_url: Option<Url>,
    request_id: Option<String>,
}

#[get("/<user>/<repo>/info/lfs/objects/batch")]
pub fn transfer(user: String, repo: String) {}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn error_serializes_correctly() {
        assert_eq!(
            serde_json::from_str::<LfsError>(
                r#"{
                "message": "Not found",
                "documentation_url": "https://lfs-server.com/docs/errors",
                "request_id": "123"
            }"#
            )
            .unwrap(),
            LfsError {
                message: "Not found".to_string(),
                documentation_url: Some(Url::parse("https://lfs-server.com/docs/errors").unwrap()),
                request_id: Some("123".to_string())
            },
        );
    }

}
