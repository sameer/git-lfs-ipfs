use actix_web::Path;
use actix_web::{
    client, error, http::header, AsyncResponder, HttpMessage, HttpRequest, HttpResponse,
};
use actix_web::{FutureResponse as ActixFutureReponse, Json};
use futures::future;
use futures::prelude::*;
use lazy_static::lazy_static;
use multiaddr::{AddrComponent, ToMultiaddr};
use multihash::Hash;
use rand::{distributions::Alphanumeric, rngs::SmallRng, FromEntropy, Rng};
use rust_base58::ToBase58;
use std::iter::FromIterator;
use url::Url;

use crate::error::Error;
use crate::spec::transfer::basic::VerifyRequest;

lazy_static! {
    static ref IPFS_PUBLIC_API_URL: Url = Url::parse("https://ipfs.io/").unwrap();
}

fn multipart_boundary() -> String {
    format!(
        "------------------------{}",
        String::from_iter(SmallRng::from_entropy().sample_iter(&Alphanumeric).take(18))
    )
}

fn multipart_begin(boundary: &str) -> String {
    format!(
        "POST /api/v0/add HTTP/1.1\r\nHost: localhost:5001\r\nContent-Type: multipart/form-data; boundary={}\r\n--{}\r\n\r\nContent-Disposition: form-data; name=\"path\"; filename=\"file\"\r\nContent-Type: application/octet-stream\r\n\r\n
",
        boundary, boundary,
    )
}

fn multipart_end(boundary: &str) -> String {
    format!("\r\n--{}--\r\n", boundary)
}

pub fn upload_object((oid, req): (Path<String>, HttpRequest)) -> ActixFutureReponse<HttpResponse> {
    let url = sha256_to_multihash(&oid)
        .map_err(|err| err.into())
        .and_then(|multihash| {
            ipfs_api_url()
                .map(|url| {
                    let mut url = url.join("api/v0/add").unwrap();
                    url.query_pairs_mut().append_pair("raw-leaves", "true");
                    url
                })
                .ok_or(Error::LocalApiUnavailableError.into())
        });
    if let Err(err) = url {
        return Box::new(future::err(err));
    }
    println!("Sending upload to {:?}", url);
    let boundary = multipart_boundary();
    client::post(url.unwrap())
        .header(
            header::CONTENT_TYPE,
            format!("{}; boundary={}", mime::MULTIPART_FORM_DATA, boundary),
        )
        .streaming(
            // req.payload()
            future::ok(bytes::Bytes::from(multipart_begin(&boundary).as_bytes()))
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
        .and_then(construct_response)
        .responder()
}

fn construct_response(
    resp: client::ClientResponse,
) -> Box<Future<Item = HttpResponse, Error = error::Error>> {
    println!("Building response");
    // Box::new(future::ok(HttpResponse::Ok().finish()))
    let mut client_resp = HttpResponse::build(resp.status());
    for (header_name, header_value) in resp.headers().iter().filter(|(h, _)| *h != "connection") {
        client_resp.header(header_name.clone(), header_value.clone());
    }
    if resp.chunked().unwrap_or(false) {
        Box::new(future::ok(client_resp.streaming(resp.payload())))
    } else {
        Box::new(
            resp.body()
                .from_err()
                .and_then(move |body| Ok(client_resp.body(body))),
        )
    }
}

pub fn download_object(oid: Path<String>) -> ActixFutureReponse<HttpResponse> {
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

pub fn verify_object(request: Json<VerifyRequest>) -> ActixFutureReponse<HttpResponse> {
    let url = sha256_to_multihash(&request.object.oid)
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
        });
    if let Err(err) = url {
        return Box::new(future::err(err));
    }
    Box::new(
        client::get(url.unwrap())
            .finish()
            .unwrap()
            .send()
            .timeout(std::time::Duration::from_secs(600))
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
