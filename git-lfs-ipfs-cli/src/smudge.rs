use std::io::{self, Read, Write};

use actix::prelude::*;
use actix_web::HttpMessage;
use futures::{future, prelude::*};

use crate::error::CliError;
use git_lfs_ipfs_lib::{ipfs, spec};

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
                    .and_then(|cid| ipfs::cat(spec::ipfs::Path::ipfs(cid)))
                    .map_err(CliError::IpfsApiError)
                    .map(|res| {
                        res.payload()
                            .map_err(git_lfs_ipfs_lib::error::Error::IpfsApiPayloadError)
                            .map_err(CliError::IpfsApiError)
                    })
                    .flatten_stream(),
            )
            .and_then(|b: bytes::Bytes, actor: &mut Self, ctx| {
                actix::fut::result(actor.stdout.write(&b).map_err(CliError::Io))
            })
            .finish()
            .then(|x, _, _| {
                System::current().stop();
                actix::fut::ok(())
            }),
        );
    }
}
