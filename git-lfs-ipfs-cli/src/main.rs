#[macro_use]
extern crate clap;

#[macro_use]
extern crate derive_more;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use actix::{prelude::*, System};

mod clean;
mod error;
mod smudge;
mod transfer;

mod ipfs;

fn main() {
    env_logger::init();
    let app_matches = clap_app!(myapp =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand smudge =>
            (about: "git-lfs smudge filter extension for ipfs")
            (@arg filename: +required "name of the file")
        )
        (@subcommand clean =>
            (about: "git-lfs clean filter extension for ipfs")
            (@arg filename: +required "name of the file")
        )
        (@subcommand transfer =>
            (about: "git-lfs custom transfer for ipfs")
        )
    )
    .get_matches();

    let sys = System::new("git-lfs-ipfs");

    match app_matches.subcommand() {
        ("smudge", _) => {
            smudge::Smudge::default().start();
        }
        ("clean", _) => {
            clean::Clean::default().start();
        }
        ("transfer", _) => {
            transfer::Transfer::default().start();
        }
        _ => {
            println!("Unknown command");
            return;
        }
    };
    sys.run();
}
