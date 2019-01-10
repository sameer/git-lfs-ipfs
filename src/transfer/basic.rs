use actix_web::Path;
use actix_web::{
    client, error, http::header, AsyncResponder, HttpMessage, HttpRequest, HttpResponse,
};
use actix_web::{FutureResponse as ActixFutureReponse, Json};
use futures::future;
use futures::prelude::*;
use rust_base58::{FromBase58, ToBase58};
use std::iter::FromIterator;
use url::Url;

use crate::error::Error;
use crate::spec::ipfs::AddResponse;
use crate::spec::transfer::basic::VerifyRequest;

pub fn upload_object((oid, req): (Path<String>, HttpRequest)) -> ActixFutureReponse<HttpResponse> {
    let mh = sha256_to_multihash(&oid);
    let url = mh.clone().map_err(|err| err.into()).and_then(|mh| {
        ipfs_api_url()
            .map(|url| {
                let mut url = url.join("api/v0/add").unwrap();
                url.query_pairs_mut()
                    .append_pair("raw-leaves", "true")
                    .append_pair("hash", "sha2-256")
                    .append_pair("cid-version", "0");
                url
            })
            .ok_or(Error::LocalApiUnavailableError.into())
    });
    if let Err(err) = url {
        return Box::new(future::err(err));
    }
    println!("Sending upload to {:?}", url);
    let boundary = multipart_boundary();
    Box::new(
        client::post(url.unwrap())
            .header(
                header::CONTENT_TYPE,
                format!("{}; boundary={}", mime::MULTIPART_FORM_DATA, boundary),
            )
            .streaming(
                // req.payload()
                future::ok(bytes::Bytes::from(
                    multipart_begin(
                        req.headers()
                            .get(header::CONTENT_LENGTH)
                            .and_then(|x| x.to_str().ok()),
                        &boundary,
                    )
                    .as_bytes(),
                ))
                .into_stream()
                .chain(req.payload())
                .chain(
                    future::ok(bytes::Bytes::from(multipart_end(&boundary).as_bytes()))
                        .into_stream(),
                ),
            )
            .unwrap()
            .send()
            .timeout(std::time::Duration::from_secs(600))
            .map_err(error::Error::from)
            .and_then(|res| res.json().map_err(error::Error::from))
            .map(move |add_response: AddResponse| {
                let res_multihash = {
                    use cid::Cid;
                    Cid::from(add_response.hash.clone()).unwrap().hash
                };
                println!("{}", add_response.hash);
                if multihash::decode(&res_multihash).unwrap().digest
                    != hex::decode(&oid.as_str()).unwrap().as_slice()
                {
                    println!(
                        "Output hash {} did not match expected {} added to IPFS as {}",
                        hex::encode(res_multihash),
                        oid,
                        add_response.hash,
                    );
                    HttpResponse::InternalServerError().finish()
                } else {
                    HttpResponse::Ok().finish()
                }
            }),
    )
}

pub fn download_object(oid: Path<String>) -> ActixFutureReponse<HttpResponse> {
    unimplemented!();
}

pub fn verify_object(request: Json<VerifyRequest>) -> ActixFutureReponse<HttpResponse> {
    let url = sha256_to_multihash(&request.object.oid)
        .map_err(|err| err.into())
        .map(|multihash| {
            // TODO: also verify size matches ls result,
            // might occur if there's hash collision
            ipfs_api_url()
                .map(|url| {
                    let mut url = url.join("api/v0/resolve").unwrap();
                    url.query_pairs_mut()
                        .append_pair("arg", &format!("/ipfs/{}", &multihash));
                    url
                })
                .unwrap_or_else(|| IPFS_PUBLIC_API_URL.clone().join(&multihash).unwrap())
        });
    if let Err(err) = url {
        return Box::new(future::err(err));
    }
    Box::new(
        client::get(url.unwrap())
            .finish()
            .unwrap()
            .send()
            .map_err(|err| err.into())
            .and_then(|res| {
                if res.status().is_success() {
                    Ok(HttpResponse::Ok().finish())
                } else {
                    Err(Error::IpfsApiError(res.status()).into())
                }
            }),
    )
}
