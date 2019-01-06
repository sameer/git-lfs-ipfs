#![feature(proc_macro_hygiene, decl_macro, custom_attribute, unrestricted_attribute_tokens)]

extern crate ipfs_api;
#[macro_use]
extern crate rocket;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate lazy_static;
extern crate url;
extern crate url_serde;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

mod pointer;
mod batch;

fn main() {
    rocket::ignite().mount("/", routes![lfs, batch::transfer]).launch();
}

#[get("/<user>/<repo>/info/lfs")]
fn lfs(user: String, repo: String) {

}

