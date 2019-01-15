use actix_web::{
    client, error, http::header, http::StatusCode, Body, FutureResponse as ActixFutureReponse,
    HttpMessage, HttpRequest, HttpResponse, Json, Path, Responder, Result as ActixResult,
};
use futures::prelude::*;
use url::Url;

use crate::error::Error;
use crate::ipfs;
use crate::spec::batch::{
    Action, Actions, BatchRequest, BatchResponse, ObjectError, ObjectResponse, Operation, Transfer,
};
use crate::spec::ipfs::Prefix;
use crate::spec::GIT_LFS_CONTENT_TYPE;

// TODO: Enable download-only capability via public IPFS gateway -- uploads will return a read-only error
// TODO: Make batch API hide actions for already uploaded objects

pub fn endpoint<'a>((path, batch): (Path<(Prefix, String)>, Json<BatchRequest>)) -> impl Responder {
    let path = path.into_inner();
    let prefix = path.0;
    let root = path.1;
    let path = ipfs::parse_ipfs_path(prefix, &root, None).wait()?;
    let base_url = Url::parse("http://localhost:5002/")
        .and_then(|url| url.join(&format!("{}/", path)))
        .and_then(|url| url.join("transfer/basic/"))
        .unwrap();
    println!("Calling batch endpoint with path {}, base url {}", path, base_url);
    // &format!(
    // "http://{}:{}/transfer/basic/",
    // req.server_settings().local_addr().ip(),
    // req.server_settings().local_addr().port()
    let download_url = base_url.join("download/").unwrap();
    let upload_url = base_url.join("upload/").unwrap();
    let verify_url = base_url.join("verify").unwrap();
    if !batch.transfer.contains(&Transfer::Basic) {
        let err: actix_web::error::Error = Error::TransferUnavailable.into();
        return Err(err);
    }

    let objects: Vec<ObjectResponse> = batch
        .objects
        .iter()
        .map(|object| match batch.operation {
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
                    ObjectError::ValidationError(),
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
                    ObjectError::ValidationError(),
                )),
        })
        .collect();
    let batch_response = BatchResponse {
        transfer: Some(Transfer::Basic),
        objects,
    };
    let json =
        serde_json::to_vec_pretty(&batch_response).map_err(|err| Error::SerializeJsonError)?;
    Ok(HttpResponse::Ok()
        .header(header::CONTENT_TYPE, GIT_LFS_CONTENT_TYPE)
        .body(json))
}
