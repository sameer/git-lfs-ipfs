use actix_web::dev::Payload;
use actix_web::{
    client, http::header, AsyncResponder, FutureResponse as ActixFutureReponse, HttpMessage,
    HttpRequest, HttpResponse, Json,
};
use bytes::Bytes;
use futures::prelude::*;
use futures::{future, stream};
use lazy_static::lazy_static;
use multihash::{Hash, Multihash};
use rand::{distributions::Alphanumeric, rngs::SmallRng, FromEntropy, Rng};
use rust_base58::{FromBase58, ToBase58};
use url::Url;

use std::iter::FromIterator;

use crate::error::Error;
use crate::spec::ipfs::*;

const EMPTY_FOLDER_HASH: &str = "QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn";

lazy_static! {
    static ref IPFS_PUBLIC_API_URL: Url = Url::parse("https://ipfs.io/").unwrap();
}

pub fn sha256_to_multihash(sha256_str: &str) -> impl Future<Item = Vec<u8>, Error = Error> {
    future::result(
        hex::decode(sha256_str)
            .ok()
            .and_then(|digest| {
                if digest.len() != 256 {
                    None
                } else {
                    multihash::encode(Hash::SHA2256, &digest).ok()
                }
            })
            .ok_or(Error::HashError),
    )
}

fn multipart_boundary() -> String {
    format!(
        "------------------------{}",
        String::from_iter(SmallRng::from_entropy().sample_iter(&Alphanumeric).take(18))
    )
}

fn multipart_begin(length: Option<u64>, boundary: &str) -> String {
    let mut begin = String::new();
    begin.push_str("POST /api/v0/add HTTP/1.1\r\nHost: localhost:5001\r\n");
    if let Some(length) = length {
        begin.push_str(&format!("Content-Length: {}\r\n", length));
    }
    begin.push_str(&format!(
        "Content-Type: multipart/form-data; boundary={}\r\n",
        boundary
    ));
    begin.push_str(&format!("--{}\r\n\r\n", boundary,));
    // begin.push_str("Content-Disposition: form-data; name=\"path\"; filename=\"file\"\r\n");
    // begin.push_str("Content-Type: application/octet-stream\r\n");
    begin
}

fn multipart_end(boundary: &str) -> String {
    format!("\r\n--{}--\r\n", boundary)
}

// req.headers()
//     .get(header::CONTENT_LENGTH)
//     .and_then(|x| x.to_str().ok()),

pub fn add(
    payload: Payload,
    length: Option<u64>,
) -> impl Future<Item = AddResponse, Error = Error> {
    ipfs_api_url()
        .map(|url| {
            let mut url = url.join("api/v0/add").unwrap();
            // url.query_pairs_mut()
            //     .append_pair("raw-leaves", "true")
            //     .append_pair("hash", "sha2-256")
            //     .append_pair("cid-version", "0");
            url
        })
        .map(|url| {
            let boundary = multipart_boundary();
            client::post(url)
                .header(
                    header::CONTENT_TYPE,
                    format!("{}; boundary={}", mime::MULTIPART_FORM_DATA, boundary),
                )
                .streaming(
                    future::ok(bytes::Bytes::from(
                        multipart_begin(length, &boundary).as_bytes(),
                    ))
                    .into_stream()
                    .chain(payload)
                    .chain(
                        future::ok(bytes::Bytes::from(multipart_end(&boundary).as_bytes()))
                            .into_stream(),
                    ),
                )
                .unwrap()
        })
        .and_then(|client| {
            client
                .send()
                .timeout(std::time::Duration::from_secs(600))
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            res.json()
                .map_err(|err| Error::IpfsApiJsonPayloadError(err))
        })
}

pub fn get<MF>(multihash: MF) -> impl Future<Item = HttpResponse, Error = Error>
where
    MF: Future<Item = Vec<u8>, Error = Error>,
{
    multihash
        .and_then(|multihash| {
            ipfs_api_url().then(|url| match url {
                Ok(url) => {
                    let mut url = url.join("api/v0/get").unwrap();
                    url.query_pairs_mut()
                        .append_pair("arg", &format!("/ipfs/{}", multihash.to_base58()));
                    Ok(url)
                }
                Err(_) => Ok(IPFS_PUBLIC_API_URL
                    .clone()
                    .join(&multihash.to_base58())
                    .unwrap()),
            })
        })
        .and_then(|url| {
            client::get(url)
                .finish()
                .unwrap()
                .send()
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            if res.status().is_success() {
                Ok(HttpResponse::Ok().streaming(res.payload()))
            } else {
                Err(Error::IpfsApiResponseError(res.status()).into())
            }
        })
}

pub fn resolve<NF>(name: NF) -> impl Future<Item = String, Error = Error>
where
    NF: Future<Item = String, Error = Error>,
{
    name.and_then(|name| {
        ipfs_api_url().then(|url| match url {
            Ok(url) => {
                let mut url = url.join("api/v0/resolve").unwrap();
                url.query_pairs_mut().append_pair("arg", &name);
                Ok(url)
            }
            Err(_) => Ok(IPFS_PUBLIC_API_URL.clone().join(&name).unwrap()),
        })
    })
    .map(|url| client::get(url).finish().unwrap())
    .and_then(|client| {
        client
            .send()
            .map_err(|err| Error::IpfsApiSendRequestError(err))
            .and_then(|res| {
                if res.status().is_success() {
                    Ok(res)
                } else {
                    Err(Error::IpfsApiResponseError(res.status()).into())
                }
            })
            .and_then(|res| res.body().map_err(|err| Error::IpfsApiPayloadError(err)))
            .map(|bytes: Bytes| String::from_utf8_lossy(&bytes).to_string())
    })
}

pub fn ls<NF>(name: NF) -> impl Future<Item = LsResponse, Error = Error>
where
    NF: Future<Item = String, Error = Error>,
{
    ipfs_api_url()
        .join(name)
        .map(|(url, name)| {
            let mut url = url.join("api/v0/ls").unwrap();
            url.query_pairs_mut().append_pair("arg", &name);

            url
        })
        .map(|url| client::get(url).finish().unwrap())
        .and_then(|client| {
            client
                .send()
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            if res.status().is_success() {
                Ok(res)
            } else {
                Err(Error::IpfsApiResponseError(res.status()).into())
            }
        })
        .and_then(|res| {
            res.json()
                .map_err(|err| Error::IpfsApiJsonPayloadError(err))
        })
}

pub fn object_patch_link<MF, NF, CF>(
    modify_multihash: MF,
    name: NF,
    add_multihash: MF,
    create: CF,
) -> impl Future<Item = ObjectResponse, Error = Error>
where
    MF: Future<Item = Vec<u8>, Error = Error>,
    NF: Future<Item = String, Error = Error>,
    CF: Future<Item = bool, Error = Error>,
{
    ipfs_api_url()
        .join5(modify_multihash, name, add_multihash, create)
        .map(|(url, modify_multihash, name, add_multihash, create)| {
            let mut url = url.join("api/v0/object/patch/add-link").unwrap();
            url.query_pairs_mut().append_pair("arg", &modify_multihash.to_base58());
            url.query_pairs_mut().append_pair("arg", &name);
            url.query_pairs_mut().append_pair("arg", &add_multihash.to_base58());
            url.query_pairs_mut().append_pair("create", create);

            url
        })
        .map(|url| client::get(url).finish().unwrap())
        .and_then(|client| {
            client
                .send()
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            if res.status().is_success() {
                Ok(res)
            } else {
                Err(Error::IpfsApiResponseError(res.status()).into())
            }
        })
        .and_then(|res| {
            res.json()
                .map_err(|err| Error::IpfsApiJsonPayloadError(err))
        })
}

pub fn name_publish<MF, KF>(multihash: MF, key: KF) -> impl Future<Item = String, Error = Error>
where
    MF: Future<Item = Vec<u8>, Error = Error>,
    KF: Future<Item = Key, Error = Error>,
{
    multihash
        .join(key)
        .and_then(|(multihash, key)| {
            ipfs_api_url().then(|url| match url {
                Ok(url) => {
                    let mut url = url.join("api/v0/name/publish").unwrap();
                    url.query_pairs_mut()
                        .append_pair("arg", &format!("/ipfs/{}", multihash.to_base58()))
                        .append_pair("key", &key.name);
                    Ok(url)
                }
                Err(_) => Ok(IPFS_PUBLIC_API_URL
                    .clone()
                    .join(&multihash.to_base58())
                    .unwrap()),
            })
        })
        .map(|url| client::get(url).finish().unwrap())
        .and_then(|client| {
            client
                .send()
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            if res.status().is_success() {
                Ok(res)
            } else {
                Err(Error::IpfsApiResponseError(res.status()).into())
            }
        })
        .and_then(|res| res.body().map_err(|err| Error::IpfsApiPayloadError(err)))
        .map(|bytes: Bytes| String::from_utf8_lossy(&bytes).to_string())
}

pub fn key_list() -> impl Future<Item = KeyListResponse, Error = Error> {
    ipfs_api_url()
        .map(|url| {
            let mut url = url.join("api/v0/key/list").unwrap();
            url
        })
        .map(|url| client::get(url).finish().unwrap())
        .and_then(|client| {
            client
                .send()
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            if res.status().is_success() {
                Ok(res)
            } else {
                Err(Error::IpfsApiResponseError(res.status()).into())
            }
        })
        .and_then(|res| {
            res.json()
                .map_err(|err| Error::IpfsApiJsonPayloadError(err))
        })
}

fn ipfs_api_url() -> impl Future<Item = Url, Error = Error> {
    use multiaddr::{AddrComponent, ToMultiaddr};
    use std::fs;
    use std::net::IpAddr;
    future::result(
        dirs::home_dir()
            .map(|mut home_dir| {
                home_dir.push(".ipfs");
                home_dir.push("api");
                home_dir
            })
            .and_then(|multiaddr_path| fs::read_to_string(&multiaddr_path).ok())
            .and_then(|multiaddr_str| multiaddr_str.to_multiaddr().ok())
            .and_then(|multiaddr| {
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
            .ok_or(Error::LocalApiUnavailableError),
    )
}
