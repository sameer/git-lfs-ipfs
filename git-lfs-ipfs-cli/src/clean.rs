use std::io::{BufRead, Write};
use std::str::FromStr;

use actix::prelude::*;
use futures::{future, prelude::*, stream};

use crate::error::CliError;
use git_lfs_ipfs_lib::{
    ipfs,
    spec::{self, transfer::custom},
};

struct ReadPayload<R: BufRead + Send>(R);

impl<R: BufRead + Send> Stream for ReadPayload<R> {
    type Item = bytes::Bytes;
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let buf = self.0.fill_buf().map(|buf| bytes::Bytes::from(buf))?;
        match buf.len() {
            0 => Ok(Async::Ready(None)),
            nonzero => {
                self.0.consume(nonzero);
                Ok(Async::Ready(Some(buf)))
            }
        }
    }
}

pub struct Clean {
    add_response: Option<Result<spec::ipfs::AddResponse, CliError>>,
}

impl Default for Clean {
    fn default() -> Self {
        Self { add_response: None }
    }
}

impl Actor for Clean {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut <Clean as Actor>::Context) {
        ctx.wait(
            actix::fut::wrap_future(ipfs::add(
                ReadPayload(std::io::BufReader::new(std::io::stdin())),
                None,
            ))
            .then(|result, actor: &mut Self, _ctx| {
                actor.add_response = Some(result.map_err(CliError::IpfsApiError));
                System::current().stop();
                actix::fut::ok(())
            }),
        );
    }

    fn stopped(&mut self, ctx: &mut <Clean as Actor>::Context) {
        if let Some(Ok(add_response)) = &self.add_response {
            debug!("{}", add_response.hash);
            match &add_response.hash {
                cid::Cid {
                    version: cid::Version::V0,
                    codec: _,
                    hash,
                } => multihash::decode(&hash).ok(),
                _ => None,
            }
            .map(|mh| match mh {
                multihash::Multihash {
                    alg: multihash::Hash::SHA2256,
                    digest,
                } => {
                    println!("{}", hex::encode(digest));
                }
                _ => {}
            });
        }
    }
}
