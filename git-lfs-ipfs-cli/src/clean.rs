use std::io::{BufRead, Write};

use actix::prelude::*;
use actix_web::HttpMessage;
use futures::prelude::*;

use crate::error::CliError;
use git_lfs_ipfs_lib::{ipfs, spec};

struct BufReadPayload<R: BufRead + Send>(R);

impl<R: BufRead + Send> Stream for BufReadPayload<R> {
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
    raw_block_data: Option<Result<bytes::Bytes, CliError>>,
}

impl Default for Clean {
    fn default() -> Self {
        Self {
            raw_block_data: None,
        }
    }
}

impl Actor for Clean {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut <Clean as Actor>::Context) {
        ctx.wait(
            actix::fut::wrap_future(
                ipfs::add(
                    BufReadPayload(std::io::BufReader::new(std::io::stdin())),
                    None,
                )
                .and_then(|add_response| ipfs::block_get(add_response.hash))
                .and_then(|res| {
                    res.body()
                        .map_err(git_lfs_ipfs_lib::error::Error::IpfsApiPayloadError)
                }),
            )
            .then(|result, actor: &mut Self, _ctx| {
                actor.raw_block_data = Some(result.map_err(CliError::IpfsApiError));
                System::current().stop();
                actix::fut::ok(())
            }),
        );
    }

    fn stopped(&mut self, ctx: &mut <Clean as Actor>::Context) {
        match &self.raw_block_data {
            Some(Ok(raw_block_data)) => std::io::stdout()
                .write_all(raw_block_data)
                .expect("unable to write to stdout"),
            Some(Err(err)) => panic!("{:?}", err),
            _ => panic!("clean stopped before completion"),
        }
    }
}
