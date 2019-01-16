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
use crate::spec::ipfs::{LsResponse, Prefix};
use crate::spec::GIT_LFS_CONTENT_TYPE;

// TODO: Enable download-only capability via public IPFS gateway -- uploads will return a read-only error
// TODO: Make batch API hide actions for already uploaded objects

pub fn endpoint<'a>(
    (path, batch): (Path<(Prefix, String)>, Json<BatchRequest>),
) -> ActixFutureReponse<HttpResponse> {
    let path = path.into_inner();
    let prefix = path.0;
    let root = path.1;

    let ls = ipfs::parse_ipfs_path(prefix.clone(), &root, None).and_then(|path| ipfs::ls(path));
    let base_url = ipfs::parse_ipfs_path(prefix, &root, None).map(|path| {
        Url::parse("http://localhost:5002/")
            .and_then(|url| url.join(&format!("{}/", path)))
            .and_then(|url| url.join("transfer/basic/"))
            .unwrap()
    });

    // &format!(
    // "http://{}:{}/transfer/basic/",
    // req.server_settings().local_addr().ip(),
    // req.server_settings().local_addr().port()

    let is_basic_supported = batch.transfer.contains(&Transfer::Basic);
    Box::new(
        ls.join(base_url)
            .and_then(move |x| {
                if !is_basic_supported {
                    return Err(Error::TransferUnavailable);
                } else {
                    return Ok(x);
                }
            })
            .map(move |(res, base_url)| {
                let download_url = base_url.join("download/").unwrap();
                let upload_url = base_url.join("upload/").unwrap();
                let verify_url = base_url.join("verify").unwrap();
                let objects: Vec<ObjectResponse> = batch
                    .objects
                    .iter()
                    .map(|object| {
                        match batch.operation {
                            Operation::Download => {
                                download_url.clone().join(&object.oid).map(|download_url| {
                                    ObjectResponse::success(
                                        object.clone(),
                                        Actions::Download {
                                            download: Action::new(download_url),
                                        },
                                    )
                                })
                            }
                            Operation::Upload => {
                                upload_url.clone().join(&object.oid).map(|upload_url| {
                                    let is_present = res
                                        .objects
                                        .iter()
                                        .nth(0)
                                        // Is this check possible? && y.size == object.size
                                        .map(|x| x.links.iter().any(|y| y.name == object.oid))
                                        .unwrap_or(false);
                                    if is_present {
                                        ObjectResponse::success(object.clone(), Actions::None)
                                    } else {
                                        ObjectResponse::success(
                                            object.clone(),
                                            Actions::UploadAndVerify {
                                                upload: Action::new(upload_url),
                                                verify: Action::new(verify_url.clone()),
                                            },
                                        )
                                    }
                                })
                            }
                        }
                        .unwrap_or(ObjectResponse::error(
                            object.clone(),
                            ObjectError::ValidationError(),
                        ))
                    })
                    .collect();
                objects
            })
            .and_then(|objects| {
                let batch_response = BatchResponse {
                    transfer: Some(Transfer::Basic),
                    objects,
                };
                let json = serde_json::to_vec_pretty(&batch_response)
                    .map_err(|err| Error::SerializeJsonError)?;
                Ok(HttpResponse::Ok()
                    .header(header::CONTENT_TYPE, GIT_LFS_CONTENT_TYPE)
                    .body(json))
            })
            .map_err(actix_web::error::Error::from),
    )
}
