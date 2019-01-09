use lazy_static::lazy_static;
use multiaddr::{AddrComponent, ToMultiaddr};
use multihash::Hash;
use reqwest::multipart;
use rocket::response::Stream;
use rocket::{http::Status, Data};
use rocket_contrib::json::Json;
use rust_base58::ToBase58;
use url::Url;

use crate::spec::transfer::basic::VerifyRequest;

lazy_static! {
    static ref IPFS_PUBLIC_API_URL: Url = Url::parse("https://ipfs.io/").unwrap();
}

#[put("/upload/<oid>", format = "binary", data = "<data>")]
pub fn upload_object(oid: String, data: Data) -> Result<(), Status> {
    let client = reqwest::Client::new();
    let form = multipart::Form::new().part("path", multipart::Part::reader(data.open()));
    client
        .post(ipfs_api_url().join("api/v0/add").unwrap())
        .query(&[("raw-leaves", "true")])
        .multipart(form)
        .send()
        .map(|_res| ())
        .map_err(|err| Status::from_code(err.status().unwrap().as_u16()).unwrap())
}

#[get("/download/<oid>")]
pub fn download_object(oid: String) -> Result<Stream<reqwest::Response>, Status> {
    let multihash = sha256_to_multihash(&oid)?;
    let client = reqwest::Client::new();
    client
        .get(ipfs_api_url().join("api/v0/get").unwrap())
        .query(&[("arg", &format!("/ipfs/{}", multihash))])
        .send()
        .map_err(|_err| Status::InternalServerError)
        .and_then(|res| {
            if res.status().is_success() {
                Ok(Stream::from(res))
            } else {
                Err(Status::from_code(res.status().as_u16()).unwrap())
            }
        })
}

#[post("/verify", format = "application/vnd.git-lfs+json", data = "<request>")]
pub fn verify_object(request: Json<VerifyRequest>) -> Result<(), Status> {
    let multihash = sha256_to_multihash(&request.object.oid)?;
    let client = reqwest::Client::new();
    // TODO: also verify size matches ls result,
    // might occur if there's hash collision
    client
        .get(ipfs_api_url().join("api/v0/ls").unwrap())
        .query(&[("arg", &format!("/ipfs/{}", multihash))])
        .send()
        .map_err(|_err| Status::InternalServerError)
        .and_then(|res| {
            if res.status().is_success() {
                Ok(())
            } else {
                Err(Status::from_code(res.status().as_u16()).unwrap())
            }
        })
}

fn sha256_to_multihash(sha256_str: &str) -> Result<String, Status> {
    base64::decode(sha256_str)
        .map_err(|_err| Status::BadRequest)
        .and_then(|hash| {
            if hash.len() != 256 {
                return Err(Status::BadRequest);
            }
            multihash::encode(Hash::SHA2256, &hash).map_err(|_err| Status::BadRequest)
        })
        .map(|hash_bytes| hash_bytes.to_base58())
}

fn ipfs_api_url() -> Url {
    use std::fs;
    use std::net::IpAddr;
    fs::read_to_string("~/.ipfs/api")
        .map_err(|_| ())
        .and_then(|api| {
            api.to_multiaddr().map_err(|_| ()).and_then(|multiaddr| {
                let mut addr: Option<IpAddr> = None;
                let mut port: Option<u16> = None;
                for addr_component in multiaddr.iter() {
                    match addr_component {
                        AddrComponent::IP4(v4addr) => addr = Some(v4addr.into()),
                        AddrComponent::IP6(v6addr) => addr = Some(v6addr.into()),
                        AddrComponent::TCP(tcpport) => port = Some(tcpport),
                        _ => {
                            return Err(());
                        }
                    }
                }
                if let (Some(addr), Some(port)) = (addr, port) {
                    Url::parse(&format!("http://{}:{}/", addr, port)).map_err(|_| ())
                } else {
                    Err(())
                }
            })
        })
        .unwrap_or_else(|_| IPFS_PUBLIC_API_URL.clone())
}
