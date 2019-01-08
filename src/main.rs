#![feature(
    proc_macro_hygiene,
    decl_macro,
    custom_attribute,
    unrestricted_attribute_tokens
)]

#[macro_use]
extern crate rocket;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate base64;
extern crate lazy_static;
extern crate multihash;
extern crate reqwest;
extern crate rocket_contrib;
extern crate rust_base58;
extern crate serde_json;
extern crate url;
extern crate url_serde;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

mod batch;
mod spec;
mod transfer;

fn main() {
    rocket::ignite()
        .mount(
            "/",
            routes![
                lfs,
                transfer::basic::verify_object,
                transfer::basic::download_object,
                transfer::basic::upload_object,
                batch::endpoint,
            ],
        )
        .launch();
}

#[get("/<user>/<repo>/info/lfs")]
fn lfs(user: String, repo: String) {}
