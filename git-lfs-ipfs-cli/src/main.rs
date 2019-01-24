extern crate cid;
extern crate dirs;
extern crate env_logger;
extern crate failure;
extern crate futures;
extern crate hex;
extern crate lazy_static;
extern crate mime;
extern crate multiaddr;
extern crate publicsuffix;
extern crate rand;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate url_serde;
#[macro_use]
extern crate log;

extern crate git_lfs_ipfs_lib;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use std::io::BufRead;

use failure::Fail;
use git_lfs_ipfs_lib::*;

use spec::transfer::custom;

#[derive(Fail, Debug)]
enum CliError {
    #[fail(display = "{}", _0)]
    SerdeJsonError(#[cause] serde_json::error::Error),
    #[fail(display = "{}", _0)]
    Io(#[cause] std::io::Error),
    #[fail(display = "Input was an unexpected event {:?}", _0)]
    UnexpectedEvent(custom::Event)
}

fn main() -> Result<(), CliError> {
    let mut stdin = std::io::BufReader::new(std::io::stdin());
    let mut buf = String::new();
    stdin.read_line(&mut buf).map_err(CliError::Io)?;
    let init: custom::Event =
        serde_json::from_str(&buf).map_err(CliError::SerdeJsonError)?;
    Ok(())
}
