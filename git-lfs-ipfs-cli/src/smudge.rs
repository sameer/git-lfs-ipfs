// use std::io;
// use std::str::FromStr;

// use actix::prelude::*;
// use futures::{future, prelude::*, stream};

// use crate::error::CliError;
// use git_lfs_ipfs_lib::{
//     ipfs,
//     spec::{self, transfer::custom},
// };

// pub struct Smudge {
//     // TODO: Does this actually need to be buffered, even if files are large?
//     stdout: io::BufWriter<io::Stdout>
// }

// impl Default for Smudge {
//     fn default() -> Self {
//         Self { stdout: io::BufWriter::new(io::stdout()) }
//     }
// }

// impl Actor for Smudge {
//     type Context = Context<Self>;
//     fn started(&mut self, ctx: &mut <Smudge as Actor>::Context) {

//     }
// }
