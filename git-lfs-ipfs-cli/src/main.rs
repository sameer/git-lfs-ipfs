extern crate actix;
#[macro_use]
extern crate clap;
extern crate dirs;
extern crate env_logger;
extern crate failure;
extern crate futures;
extern crate hex;
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate multihash;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate url_serde;

extern crate git_lfs_ipfs_lib;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use actix::prelude::*;

mod clean;
mod error;
mod smudge;
mod transfer;

fn main() {
    env_logger::init();
    let app_matches = clap_app!(myapp =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand smudge =>
            (about: "git-lfs smudge filter extension for ipfs")
        )
        (@subcommand clean =>
            (about: "git-lfs clean filter extension for ipfs")
        )
        (@subcommand transfer =>
            (about: "git-lfs custom transfer for ipfs")
        )
    )
    .get_matches();

    let sys = System::new("git-lfs-ipfs");

    match app_matches.subcommand() {
        ("smudge", _) => {
            std::io::copy(&mut std::io::stdin(), &mut std::io::stdout())
                .expect("input could not be echoed");
        }
        ("clean", _) => {
            clean::Clean::default().start();
        }
        ("transfer", _) => {
            transfer::Transfer::default().start();
        }
        _ => {
            info!("Unknown command");
        }
    };
    sys.run();
}
