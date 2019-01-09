#![feature(
    proc_macro_hygiene,
    decl_macro,
    custom_attribute,
    unrestricted_attribute_tokens
)]

extern crate base64;
extern crate dirs;
extern crate failure;
extern crate futures;
extern crate lazy_static;
extern crate mpart_async;
extern crate multiaddr;
extern crate multihash;
extern crate rust_base58;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate url_serde;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use actix_web::App;

mod batch;
mod spec;
mod transfer;

fn main() {
    actix_web::server::new(|| {
        App::new()
            .resource("/verify", |r| {
                r.post().with_async(transfer::basic::verify_object)
            })
            .resource("/download/{oid}", |r| {
                r.get().with_async(transfer::basic::download_object)
            })
            .resource("/upload/{oid}", |r| {
                r.put().with_async(transfer::basic::upload_object)
            })
            .resource("/objects/batch", |r| r.post().with(batch::endpoint))
    })
    .bind("127.0.0.1:5002")
    .unwrap()
    .run();
}

#[get("/<user>/<repo>/info/lfs")]
fn lfs(user: String, repo: String) {}
