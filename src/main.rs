extern crate base64;
extern crate dirs;
extern crate env_logger;
extern crate failure;
extern crate futures;
extern crate hyper_multipart_rfc7578;
extern crate lazy_static;
extern crate mime;
extern crate multiaddr;
extern crate multihash;
extern crate rust_base58;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate url_serde;
extern crate rand;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use actix_web::{http::header, middleware::Logger, pred, App};

mod batch;
mod error;
mod spec;
mod transfer;

fn main() {
    env_logger::init();
    actix_web::server::new(|| {
        vec![
            App::new()
                .prefix("/transfer/basic/")
                .middleware(Logger::default())
                .resource("/verify", |r| {
                    r.post()
                        .filter(pred::Header(
                            header::CONTENT_TYPE.as_str(),
                            spec::GIT_LFS_CONTENT_TYPE,
                        ))
                        .with_async(transfer::basic::verify_object)
                })
                .resource("/download/{oid}", |r| {
                    r.get().with_async(transfer::basic::download_object)
                })
                .resource("/upload/{oid}", |r| {
                    r.put()
                        // .filter(pred::Header(
                        //     header::CONTENT_TYPE.as_str(),
                        //     mime::OCTET_STREAM.as_str(),
                        // ))
                        .with_async(transfer::basic::upload_object)
                })
                .boxed(),
            App::new()
                .middleware(Logger::default())
                .resource("/objects/batch", |r| {
                    r.post()
                        // .filter(pred::Header(
                        //     header::CONTENT_TYPE.as_str(),
                        //     spec::GIT_LFS_CONTENT_TYPE,
                        // ))
                        .with(batch::endpoint)
                })
                .boxed(),
        ]
    })
    .bind("127.0.0.1:5002")
    .unwrap()
    .run();
}
