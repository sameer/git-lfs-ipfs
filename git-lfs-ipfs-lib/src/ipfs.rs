use actix_web::dev::Payload;
use actix_web::{
    client, dev::HttpResponseBuilder, http::header, AsyncResponder,
    FutureResponse as ActixFutureReponse, HttpMessage, HttpRequest, HttpResponse, Json,
};
use bytes::Bytes;
use cid::Cid;
use futures::prelude::*;
use futures::{future, stream};
use lazy_static::lazy_static;
use rand::{distributions::Alphanumeric, rngs::SmallRng, FromEntropy, Rng};
use url::Url;

use std::io::Write;
use std::iter::FromIterator;
use std::str::FromStr;
use std::time::Duration;

use crate::error::Error;
use crate::spec::ipfs::*;

lazy_static! {
    static ref IPFS_PUBLIC_API_URL: Url = Url::parse("https://ipfs.io/").unwrap();
}

pub fn sha256_to_cid(sha256_str: &str) -> impl Future<Item = Cid, Error = Error> {
    future::result(
        hex::decode(sha256_str)
            .ok()
            .and_then(|digest| {
                if digest.len() != 32 {
                    None
                } else {
                    Some(Cid::new(cid::Codec::Raw, cid::Version::V0, &digest))
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

pub fn parse_ipfs_path<I>(
    prefix: Prefix,
    root: &str,
    suffix: I,
) -> impl Future<Item = Path, Error = Error>
where
    I: Into<Option<std::path::PathBuf>>,
{
    future::result(Root::from_str(root).and_then(|root| {
        Path::parse(prefix, root, suffix.into()).ok_or(Error::IpfsPathParseError("Parse failed"))
    }))
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
        .map(move |url| {
            let boundary = multipart_boundary();
            debug!("Sending add request to {}", url);
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
                .timeout(Duration::from_secs(600))
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            res.json()
                .map_err(|err| Error::IpfsApiJsonPayloadError(err))
        })
    // .and_then(|res: Result<AddResponse>| match res {
    //     Result::Ok(res) => Ok(res),
    //     Result::Err(err) => Err(Error::IpfsApiResponseError(err)),
    // })
}

pub fn get(path: Path) -> impl Future<Item = HttpResponse, Error = Error> {
    ipfs_api_url()
        .map(move |url| {
            let mut url = url.join("api/v0/get").unwrap();
            url.query_pairs_mut().append_pair("arg", &path.to_string());
            url
        })
        .and_then(|url| {
            debug!("Sending get request to {}", url);
            client::get(url)
                .finish()
                .unwrap()
                .send()
                .timeout(Duration::from_secs(600))
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        // TODO: Handle json error responses
        .and_then(|res| {
            // if res.status().is_success() {
            let mut proxy_res: HttpResponseBuilder = HttpResponse::build(res.status());
            res.headers()
                .iter()
                .filter(|(h, _)| *h != "connection")
                .for_each(|(k, v)| {
                    proxy_res.header(k.clone(), v.clone());
                });
            Ok(proxy_res.streaming(res.payload()))
            // }
            // else {
            //     Err(res.json().map_err(|err| Error::IpfsApiJsonPayloadError(err)))
            // }
        })
}

pub fn cat_to_fs(path: Path, output: std::path::PathBuf) -> impl Future<Item = (), Error = Error> {
    ipfs_api_url()
        .map(move |url| {
            let mut url = url.join("api/v0/cat").unwrap();
            url.query_pairs_mut().append_pair("arg", &path.to_string());
            url
        })
        .and_then(|url| {
            debug!("Sending cattofs request to {}", url);
            client::get(url)
                .finish()
                .unwrap()
                .send()
                .timeout(Duration::from_secs(600))
                .map_err(Error::IpfsApiSendRequestError)
        })
        .and_then(|res| {
            res.payload()
                .map_err(Error::IpfsApiPayloadError)
                .fold(std::fs::File::create(output).unwrap(), |mut acc, b| {
                    acc.write(&b).unwrap();
                    future::ok(acc)
                })
                .and_then(|_| Ok(()))
        })
}

pub fn cat(path: Path) -> impl Future<Item = HttpResponse, Error = Error> {
    ipfs_api_url()
        .then(move |url| match url {
            Ok(url) => {
                let mut url = url.join("api/v0/cat").unwrap();
                url.query_pairs_mut().append_pair("arg", &path.to_string());
                Ok(url)
            }
            Err(_) => Ok(IPFS_PUBLIC_API_URL.clone().join(&path.to_string()).unwrap()),
        })
        .and_then(|url| {
            debug!("Sending cat request to {}", url);
            client::get(url)
                .finish()
                .unwrap()
                .send()
                .timeout(Duration::from_secs(600))
                .map_err(Error::IpfsApiSendRequestError)
        })
        // TODO: Handle json error responses
        .and_then(|res| {
            // if res.status().is_success() {
            let mut proxy_res: HttpResponseBuilder = HttpResponse::build(res.status());
            res.headers()
                .iter()
                .filter(|(h, _)| *h != "connection")
                .for_each(|(k, v)| {
                    proxy_res.header(k.clone(), v.clone());
                });
            Ok(proxy_res.streaming(res.payload()))
            // }
            // else {
            //     Err(res.json().map_err(|err| Error::IpfsApiJsonPayloadError(err)))
            // }
        })
}

pub fn resolve(path: Path) -> impl Future<Item = Cid, Error = Error> {
    ipfs_api_url()
        .then(move |url| match url {
            Ok(url) => {
                let mut url = url.join("api/v0/resolve").unwrap();
                url.query_pairs_mut().append_pair("arg", &path.to_string());
                debug!("Sending resolve request to {}", url);
                Ok(url)
            }
            Err(_) => Ok(IPFS_PUBLIC_API_URL.clone().join(&path.to_string()).unwrap()),
        })
        .map(|url| client::get(url).finish().unwrap())
        .and_then(|client| {
            client
                .send()
                .timeout(Duration::from_secs(600))
                .map_err(|err| Error::IpfsApiSendRequestError(err))
                .and_then(|res| {
                    res.json().map_err(|err| {
                        error!("{:?}", err);
                        Error::IpfsApiJsonPayloadError(err)
                    })
                })
                // .and_then(|res: Result<ResolveResponse>| match res {
                //     Result::Ok(res) => Ok(res),
                //     Result::Err(err) => Err(Error::IpfsApiResponseError(err)),
                // })
                .and_then(|res: ResolveResponse| match res.path.root {
                    Root::Cid(cid) => Ok(cid),
                    Root::DnsLink(_link) => Err(Error::IpfsPathParseError("Expected CID")),
                })
        })
}

pub fn ls(path: Path) -> impl Future<Item = LsResponse, Error = Error> {
    ipfs_api_url()
        .wait()
        .map(move |url| {
            let mut url = url.join("api/v0/ls").unwrap();
            url.query_pairs_mut().append_pair("arg", &path.to_string());
            debug!("Sending ls request to {}", url);
            url
        })
        .map(|url| client::get(url).finish().unwrap())
        .unwrap()
        .send()
        .timeout(Duration::from_secs(600))
        .map_err(|err| Error::IpfsApiSendRequestError(err))
        .and_then(|res| {
            res.json()
                .map_err(|err| Error::IpfsApiJsonPayloadError(err))
        })
    // .and_then(|res: Result<LsResponse>| match res {
    //     Result::Ok(res) => Ok(res),
    //     Result::Err(err) => Err(Error::IpfsApiResponseError(err)),
    // })
}

pub fn object_patch_link(
    modify_cid: Cid,
    name: String,
    add_cid: Cid,
    create: bool,
) -> impl Future<Item = ObjectResponse, Error = Error> {
    ipfs_api_url()
        .map(move |url| {
            let mut url = url.join("api/v0/object/patch/add-link").unwrap();
            url.query_pairs_mut()
                .append_pair("arg", &modify_cid.to_string());
            url.query_pairs_mut().append_pair("arg", &name);
            url.query_pairs_mut()
                .append_pair("arg", &add_cid.to_string());
            url.query_pairs_mut()
                .append_pair("create", &create.to_string());
            debug!("Sending object patch link request to {}", url);

            url
        })
        .map(|url| client::get(url).finish().unwrap())
        .and_then(|client| {
            client
                .send()
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            res.json()
                .map_err(|err| Error::IpfsApiJsonPayloadError(err))
        })
    // .and_then(|res: Result<ObjectResponse>| match res {
    //     Result::Ok(res) => Ok(res),
    //     Result::Err(err) => Err(Error::IpfsApiResponseError(err)),
    // })
}

pub fn name_publish(cid: Cid, key: Key) -> impl Future<Item = String, Error = Error> {
    debug!("Publishing with key {:?}", key);
    ipfs_api_url()
        .then(move |url| match url {
            Ok(url) => {
                let mut url = url.join("api/v0/name/publish").unwrap();
                url.query_pairs_mut()
                    .append_pair("arg", &format!("/ipfs/{}", cid))
                    .append_pair("key", &key.name);
                debug!("Sending name publish request to {}", url);
                Ok(url)
            }
            Err(_) => Ok(IPFS_PUBLIC_API_URL.clone().join(&cid.to_string()).unwrap()),
        })
        .map(|url| client::get(url).finish().unwrap())
        .and_then(|client| {
            client
                .send()
                .timeout(Duration::from_secs(600))
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| res.body().map_err(|err| Error::IpfsApiPayloadError(err)))
        .map(|bytes: Bytes| String::from_utf8_lossy(&bytes).to_string())
}

pub fn key_list() -> impl Future<Item = KeyListResponse, Error = Error> {
    ipfs_api_url()
        .map(|url| {
            let mut url = url.join("api/v0/key/list").unwrap();
            debug!("Sending key list request to {}", url);
            url
        })
        .map(|url| client::get(url).finish().unwrap())
        .and_then(|client| {
            client
                .send()
                .map_err(|err| Error::IpfsApiSendRequestError(err))
        })
        .and_then(|res| {
            res.json()
                .map_err(|err| Error::IpfsApiJsonPayloadError(err))
        })
    // .and_then(|res: Result<KeyListResponse>| match res {
    //     Result::Ok(res) => Ok(res),
    //     Result::Err(err) => {
    //         error!("{:?}", err);
    //         Err(Error::IpfsApiResponseError(err))
    //     }
    // })
}

pub fn ipfs_api_url() -> impl Future<Item = Url, Error = Error> {
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
