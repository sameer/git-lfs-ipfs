use std::io::{self, Read, Write};

use actix::prelude::*;
use futures::{future, prelude::*};
use git_lfs_ipfs_lib::spec;

use crate::error::CliError;

pub struct Smudge {
    // TODO: Does this actually need to be buffered, even if files are large?
    stdout: io::BufWriter<io::Stdout>,
}

impl Default for Smudge {
    fn default() -> Self {
        Self {
            stdout: io::BufWriter::new(io::stdout()),
        }
    }
}

impl Actor for Smudge {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut <Smudge as Actor>::Context) {
        let mut raw_object = Vec::with_capacity(8192);
        io::stdin()
            .read_to_end(&mut raw_object)
            .expect("could not read raw object from stdin");
        ctx.wait(
            actix::fut::wrap_stream(
                future::ok(multihash::encode(multihash::Hash::SHA2256, &raw_object).unwrap())
                    .map(|mh| cid::Cid::new(cid::Codec::DagProtobuf, cid::Version::V0, &mh))
                    .map(|cid| {
                        ipfs_api::IpfsClient::default()
                            .cat(&spec::ipfs::Path::ipfs(cid).to_string())
                    })
                    .flatten_stream()
                    .map_err(CliError::IpfsApiError),
            )
            .and_then(|b: bytes::Bytes, actor: &mut Self, ctx| {
                actix::fut::result(actor.stdout.write_all(&b).map_err(CliError::Io))
            })
            .finish()
            .then(|x, _, _| match x {
                Ok(_) => {
                    System::current().stop();
                    actix::fut::ok(())
                }
                Err(err) => panic!("{:?}", err),
            }),
        );
    }
}
