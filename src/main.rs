extern crate cid;
extern crate dirs;
extern crate env_logger;
extern crate failure;
extern crate futures;
extern crate hex;
extern crate lazy_static;
extern crate mime;
extern crate multiaddr;
extern crate rand;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate url_serde;
extern crate publicsuffix;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use actix_web::{http::header, middleware::Logger, pred, App};

mod batch;
mod error;
mod ipfs;
mod spec;
mod transfer;

fn main() {
    env_logger::init();
    actix_web::server::new(|| {
        vec![
            App::new()
                .middleware(Logger::default())
                .resource("/{prefix}/{QmHash}/transfer/basic/verify", |r| {
                    r.post()
                        .filter(pred::Header(
                            header::CONTENT_TYPE.as_str(),
                            spec::GIT_LFS_CONTENT_TYPE,
                        ))
                        .with_async(transfer::basic::verify_object)
                })
                .resource("/{prefix}/{QmHash}/transfer/basic/download/{oid}", |r| {
                    r.get().with_async(transfer::basic::download_object)
                })
                .resource("/{prefix}/{QmHash}/transfer/basic/upload/{oid}", |r| {
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
                .resource("/{prefix}/{QmHash}/objects/batch", |r| {
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
