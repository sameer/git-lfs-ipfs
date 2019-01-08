use multihash::Hash;
use reqwest::multipart;
use rocket::response::Stream;
use rocket::{http::Status, Data};
use rocket_contrib::json::Json;
use rust_base58::ToBase58;

use crate::spec::transfer::basic::VerifyRequest;
use crate::spec::Object;

#[put("/upload/<oid>", format = "binary", data = "<data>")]
pub fn upload_object(oid: String, data: Data) -> Result<(), Status> {
    let client = reqwest::Client::new();
    let form = multipart::Form::new().part("path", multipart::Part::reader(data.open()));
    client
        .post("http://localhost:5001/api/v0/add")
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
        .get("http://localhost:5001/api/v0/get")
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
        .get("http://localhost:5001/api/v0/ls")
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
