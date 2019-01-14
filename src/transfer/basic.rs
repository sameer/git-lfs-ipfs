use actix_web::Path;
use actix_web::{
    client, error::Error, http::header, AsyncResponder, HttpMessage, HttpRequest, HttpResponse,
};
use actix_web::{FutureResponse as ActixFutureReponse, Json};
use futures::future;
use futures::prelude::*;
use rust_base58::{FromBase58, ToBase58};
use std::iter::FromIterator;
use url::Url;

use crate::ipfs;
use crate::spec::ipfs::{AddResponse, IpfsPath, KeyListResponse, Prefix};
use crate::spec::transfer::basic::VerifyRequest;

pub fn upload_object(
    (prefix, path_type, oid, req): (Path<Prefix>, Path<String>, Path<String>, HttpRequest),
) -> ActixFutureReponse<HttpResponse> {
    // let object_hash = ipfs::add(req.payload(), None);
    // let current_hash = ipfs::resolve(
    //     ipfs::parse_ipfs_path(prefix.into_inner(), &path_type).map(|path| format!("{}", path)),
    // );
    // let new_hash = ipfs::object_patch_link(
    //     current_hash,
    //     future::ok(oid.into_inner()),
    //     current_hash,
    //     future::ok(false),
    // );
    // let key = ipfs::key_list()
    //     .join(
    //         ipfs::parse_ipfs_path(prefix.into_inner(), &path_type)
    //             .map(|path: IpfsPath| format!("{}", path.path_type)),
    //     )
    //     .and_then(|(key_list_response, path_type): (KeyListResponse, String)| {
    //         key_list_response.keys.iter().find(|x| x.id == path_type)
    //     });
    // Box::new(
    //     ipfs::name_publish(new_hash, key)
    //         .map(|_| HttpResponse::Ok().finish())
    //         .map_err(Error::from),
    // )
    unimplemented!();
}

pub fn download_object(oid: Path<String>) -> ActixFutureReponse<HttpResponse> {
    unimplemented!();
}

pub fn verify_object(request: Json<VerifyRequest>) -> ActixFutureReponse<HttpResponse> {
    unimplemented!();
}
