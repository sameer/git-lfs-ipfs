use actix_web::{client, error, http::StatusCode, Body, HttpMessage, HttpRequest, HttpResponse};
use actix_web::{FutureResponse as ActixFutureReponse, Json};
use failure::Fail;
use futures::future;
use futures::prelude::*;
use lazy_static::lazy_static;
use multiaddr::{AddrComponent, ToMultiaddr};
use multihash::Hash;
use rust_base58::ToBase58;
use url::Url;

use crate::spec::transfer::basic::VerifyRequest;

lazy_static! {
    static ref IPFS_PUBLIC_API_URL: Url = Url::parse("https://ipfs.io/").unwrap();
}

pub fn upload_object<S: 'static>(
    oid: String,
    req: HttpRequest<S>,
) -> ActixFutureReponse<HttpResponse> {
    Box::new(
        future::result(sha256_to_multihash(&oid))
            .map_err(|err| err.into())
            .and_then(|multihash| {
                ipfs_api_url()
                    .map(|url| {
                        let mut url = url.join("api/v0/add").unwrap();
                        url.query_pairs_mut().append_pair("raw-leaves", "true");
                        url
                    })
                    .ok_or(Error::LocalApiUnavailableError.into())
            })
            .and_then(|url| {
                let mut mpart = mpart_async::MultipartRequest::default();
                mpart.add_stream("path", "", "application/octet-stream", req.payload());
                client::post(url)
                    .body(Body::Streaming(Box::new(mpart.map_err(|err| err.into()))))
                    .unwrap()
                    .send()
                    .map_err(|err| err.into())
                    .map(|_res| HttpResponse::Ok().finish())
            }),
    )
}

#[get("/download/<oid>")]
pub fn download_object(oid: String) -> ActixFutureReponse<HttpResponse> {
    Box::new(
        future::result(sha256_to_multihash(&oid))
            .map_err(|err| err.into())
            .map(|multihash| {
                ipfs_api_url()
                    .map(|url| {
                        let mut url = url.join("api/v0/get").unwrap();
                        url.query_pairs_mut()
                            .append_pair("arg", &format!("/ipfs/{}", &multihash));
                        url
                    })
                    .unwrap_or_else(|| IPFS_PUBLIC_API_URL.clone().join(&multihash).unwrap())
            })
            .and_then(|url| {
                client::get(url)
                    .finish()
                    .unwrap()
                    .send()
                    .map_err(|err| err.into())
                    .and_then(|res| {
                        if res.status().is_success() {
                            Ok(HttpResponse::Ok().streaming(res.payload()))
                        } else {
                            Err(Error::IpfsApiError(res.status()).into())
                        }
                    })
            }),
    )
}

#[post("/verify", format = "application/vnd.git-lfs+json", data = "<request>")]
pub fn verify_object(request: Json<VerifyRequest>) -> ActixFutureReponse<HttpResponse> {
    Box::new(
        future::result(sha256_to_multihash(&request.object.oid))
            .map_err(|err| err.into())
            .map(|multihash| {
                // TODO: also verify size matches ls result,
                // might occur if there's hash collision
                ipfs_api_url()
                    .map(|url| {
                        let mut url = url.join("api/v0/ls").unwrap();
                        url.query_pairs_mut()
                            .append_pair("arg", &format!("/ipfs/{}", &multihash));
                        url
                    })
                    .unwrap_or_else(|| IPFS_PUBLIC_API_URL.clone().join(&multihash).unwrap())
            })
            .and_then(|url| {
                client::get(url.into_string())
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
                    })
            }),
    )
}

#[derive(Fail, Debug)]
enum Error {
    #[fail(display = "A bad SHA2-256 hash was provided.")]
    HashError,
    #[fail(
        display = "A local API could not be found, and the public API does not support this functionality."
    )]
    LocalApiUnavailableError,
    #[fail(display = "An error was encountered in a request to the IPFS API")]
    IpfsApiError(StatusCode),
}

impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match *self {
            Error::HashError => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::LocalApiUnavailableError => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            Error::IpfsApiError(status) => HttpResponse::new(status),
        }
    }
}

fn sha256_to_multihash(sha256_str: &str) -> Result<String, Error> {
    base64::decode(sha256_str)
        .ok()
        .and_then(|hash| multihash::encode(Hash::SHA2256, &hash).ok())
        .map(|hash_bytes| hash_bytes.to_base58())
        .ok_or(Error::HashError)
}

fn ipfs_api_url() -> Option<Url> {
    use std::fs;
    use std::net::IpAddr;
    dirs::home_dir()
        .map(|mut home_dir| {
            home_dir.push(".ipfs");
            home_dir.push("api");
            home_dir
        })
        .and_then(|path| fs::read_to_string(&path).ok())
        .and_then(|api| {
            api.to_multiaddr().ok().and_then(|multiaddr| {
                let mut addr: Option<IpAddr> = None;
                let mut port: Option<u16> = None;
                for addr_component in multiaddr.iter() {
                    match addr_component {
                        AddrComponent::IP4(v4addr) => addr = Some(v4addr.into()),
                        AddrComponent::IP6(v6addr) => addr = Some(v6addr.into()),
                        AddrComponent::TCP(tcpport) => port = Some(tcpport),
                        _ => {
                            return None;
                        }
                    }
                }
                if let (Some(addr), Some(port)) = (addr, port) {
                    Url::parse(&format!("http://{}:{}/", addr, port))
                        .map_err(|_| ())
                        .ok()
                } else {
                    None
                }
            })
        })
}
