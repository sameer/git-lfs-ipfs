use actix_web::Path;
use actix_web::{FutureResponse as ActixFutureReponse, Json};
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use futures::future;
use futures::prelude::*;

use crate::error::Error;
use crate::ipfs;
use crate::spec::ipfs::{
    AddResponse, KeyListResponse, Link, LsResponse, Object, ObjectResponse, Path as IpfsPath,
    Prefix, Root,
};
use crate::spec::transfer::basic::VerifyRequest;

use std::path::PathBuf;

pub fn upload_object(
    (path, req): (Path<(Prefix, String, String)>, HttpRequest),
) -> ActixFutureReponse<HttpResponse> {
    let path = path.into_inner();
    let prefix = path.0;
    let root = path.1;
    let oid = path.2;
    let object_hash = ipfs::add(req.payload(), None).map(|res: AddResponse| res.hash);
    let current_cid = ipfs::resolve(ipfs::parse_ipfs_path(
        prefix.clone(),
        &root,
        PathBuf::from(oid.clone()),
    ));
    let new_cid = ipfs::object_patch_link(
        current_cid,
        ipfs::sha256_to_cid(&oid),
        object_hash,
        future::ok(false),
    )
    .map(|x: ObjectResponse| x.object.hash);
    let key = ipfs::key_list()
        .join(ipfs::parse_ipfs_path(prefix, &root, PathBuf::from(oid)))
        .and_then(
            |(mut key_list_response, path): (KeyListResponse, IpfsPath)| {
                debug!("Finding matching key in {:?}", key_list_response);
                key_list_response
                    .keys
                    .drain(..)
                    .find(|x| match &path.root {
                        Root::Cid(cid) => cid.hash == x.id.hash,
                        _ => false,
                    })
                    .ok_or(Error::IpfsUploadNotPossible)
            },
        );
    Box::new(
        ipfs::name_publish(new_cid, key)
            .map(|_| HttpResponse::Ok().finish())
            .map_err(actix_web::error::Error::from),
    )
}

pub fn download_object(path: Path<(Prefix, String, String)>) -> ActixFutureReponse<HttpResponse> {
    let path = path.into_inner();
    let prefix = path.0;
    let root = path.1;
    let oid = path.2;
    Box::new(
        ipfs::get(ipfs::parse_ipfs_path(
            prefix,
            &root,
            PathBuf::from(oid),
        ))
        .map_err(actix_web::error::Error::from),
    )
}

pub fn verify_object(
    (path, request): (Path<(Prefix, String)>, Json<VerifyRequest>),
) -> ActixFutureReponse<HttpResponse> {
    let path = path.into_inner();
    let prefix = path.0;
    let root = path.1;
    let oid = request.into_inner().object.oid;
    Box::new(
        ipfs::ls(ipfs::parse_ipfs_path(
            prefix,
            &root,
            PathBuf::from(oid.clone()),
        ))
        .and_then(|mut res: LsResponse| {
            res.objects
                .drain(..)
                .nth(0)
                .map(move |mut x: Object| x.links.drain(..).find(|y: &Link| y.name == oid))
                .ok_or(Error::VerifyFailed)
        })
        .map(|_link| HttpResponse::Ok().finish())
        .map_err(actix_web::error::Error::from),
    )
}
