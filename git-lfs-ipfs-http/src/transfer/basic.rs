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
    let current_cid = ipfs::parse_ipfs_path(prefix.clone(), &root, PathBuf::from(oid.clone()))
        .and_then(|path| ipfs::resolve(path));
    let key =
        ipfs::parse_ipfs_path(prefix.clone(), &root, PathBuf::from(oid.clone())).and_then(|path| {
            ipfs::key_list().and_then(move |mut key_list_response: KeyListResponse| {
                debug!("Looking for key {}", path.root);
                key_list_response
                    .keys
                    .iter()
                    .for_each(|key| debug!("Checking key {} with hash {}", key.name, key.id));
                key_list_response
                    .keys
                    .drain(..)
                    .find(|x| match &path.root {
                        Root::Cid(cid) => cid.hash == x.id.hash,
                        _ => false,
                    })
                    .map(|x| {
                        debug!("Found matching key!");
                        x
                    })
                    .ok_or(Error::IpfsUploadNotPossible)
            })
        });
    let new_cid = current_cid
        .join(object_hash)
        .and_then(move |(current_cid, object_hash)| {
            ipfs::object_patch_link(current_cid, oid.clone(), object_hash, false)
        })
        .map(|x: ObjectResponse| x.object.hash);
    Box::new(
        new_cid
            .join(key)
            .and_then(|(new_cid, key)| ipfs::name_publish(new_cid, key))
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
        ipfs::parse_ipfs_path(prefix, &root, PathBuf::from(oid))
            .and_then(|path| ipfs::cat(path))
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
        ipfs::parse_ipfs_path(prefix, &root, PathBuf::from(oid.clone()))
            .and_then(|path| ipfs::ls(path))
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
