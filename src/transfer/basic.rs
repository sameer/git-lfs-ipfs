use actix_web::Path;
use actix_web::{FutureResponse as ActixFutureReponse, Json};
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use futures::future;
use futures::prelude::*;

use crate::error::Error;
use crate::ipfs;
use crate::spec::ipfs::{AddResponse, IpfsPath, KeyListResponse, ObjectResponse, PathType, Prefix};
use crate::spec::transfer::basic::VerifyRequest;

pub fn upload_object(
    (prefix, path_type, oid, req): (Path<Prefix>, Path<String>, Path<String>, HttpRequest),
) -> ActixFutureReponse<HttpResponse> {
    let prefix = prefix.into_inner();
    let object_hash = ipfs::add(req.payload(), None).map(|res: AddResponse| res.hash);
    let current_cid = ipfs::resolve(ipfs::parse_ipfs_path(prefix.clone(), &path_type));
    let new_cid = ipfs::object_patch_link(
        current_cid,
        ipfs::sha256_to_cid(&oid),
        object_hash,
        future::ok(false),
    )
    .map(|x: ObjectResponse| x.object.hash);
    let key = ipfs::key_list()
        .join(ipfs::parse_ipfs_path(prefix, &path_type))
        .and_then(|(key_list_response, path): (KeyListResponse, IpfsPath)| {
            key_list_response
                .keys
                .drain(..)
                .find(|x| PathType::Cid(x.id) == path.path_type)
                .ok_or(Error::IpfsUploadNotPossible)
        });
    Box::new(
        ipfs::name_publish(new_cid, key)
            .map(|_| HttpResponse::Ok().finish())
            .map_err(actix_web::error::Error::from),
    )
}

pub fn download_object(oid: Path<String>) -> ActixFutureReponse<HttpResponse> {
    unimplemented!();
}

pub fn verify_object(request: Json<VerifyRequest>) -> ActixFutureReponse<HttpResponse> {
    unimplemented!();
}
