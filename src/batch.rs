use rocket::http::{ContentType, Status};
use rocket::response::Result as RocketResult;
use rocket::Config;
use rocket::Response;
use rocket::State;
use rocket_contrib::json::Json;
use url::Url;

use crate::spec::batch::{
    Action, Actions, BatchRequest, BatchResponse, ObjectError, ObjectResponse, Operation, Transfer,
};

// TODO: Enable download-only capability via public IPFS gateway -- uploads will return a read-only error

#[post(
    "/objects/batch",
    format = "application/vnd.git-lfs+json",
    data = "<request>"
)]
pub fn endpoint<'a>(request: Json<BatchRequest>, config: State<Config>) -> RocketResult<'a> {
    let base_url = Url::parse(&format!("http://{}:{}/", config.address, config.port)).unwrap();
    let download_url = base_url.join("download/").unwrap();
    let upload_url = base_url.join("upload/").unwrap();
    let verify_url = base_url.join("verify/").unwrap();
    if !request.transfer.contains(&Transfer::Basic) {
        return Err(Status::NotImplemented);
    }

    let objects: Vec<ObjectResponse> = request
        .objects
        .iter()
        .map(|object| match request.operation {
            Operation::Download => download_url
                .clone()
                .join(&object.oid)
                .map(|download_url| {
                    ObjectResponse::success(
                        object.clone(),
                        Actions::download(Action::new(download_url)),
                    )
                })
                .unwrap_or(ObjectResponse::error(
                    object.clone(),
                    ObjectError::VALIDATION_ERROR,
                )),
            Operation::Upload => upload_url
                .clone()
                .join(&object.oid)
                .map(|upload_url| {
                    ObjectResponse::success(
                        object.clone(),
                        Actions::upload_and_verify(
                            Action::new(upload_url),
                            Action::new(verify_url.clone()),
                        ),
                    )
                })
                .unwrap_or(ObjectResponse::error(
                    object.clone(),
                    ObjectError::VALIDATION_ERROR,
                )),
        })
        .collect();
    let batch_response = BatchResponse {
        transfer: Some(Transfer::Basic),
        objects,
    };
    let json =
        serde_json::to_vec_pretty(&batch_response).map_err(|_| Status::InternalServerError)?;
    Ok(Response::build()
        .header(ContentType::new("application", "vnd.git-lfs+json"))
        .sized_body(std::io::Cursor::new(json))
        .finalize())
}
