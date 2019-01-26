extern crate cid;
extern crate dirs;
extern crate env_logger;
extern crate failure;
extern crate futures;
extern crate hex;
extern crate lazy_static;
extern crate mime;
extern crate multiaddr;
extern crate multihash;
extern crate publicsuffix;
extern crate rand;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate url_serde;
#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

pub mod error;
pub mod ipfs;
pub mod pointer;
pub mod spec;
