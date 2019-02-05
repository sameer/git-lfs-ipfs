use std::io::{self, BufRead, Write};

use actix::prelude::*;
use actix_web::HttpMessage;
use futures::{future, prelude::*, sync::mpsc};

use crate::error::CliError;
use git_lfs_ipfs_lib::ipfs;

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
        let (tx, rx) = mpsc::channel(4);
        let stdin = io::stdin();
        actix::spawn(
            future::loop_fn(tx, move |tx| {
                let mut lock = stdin.lock();
                let buf = lock.fill_buf().map(bytes::Bytes::from);
                let mut should_break = false;
                if let Ok(buf) = &buf {
                    lock.consume(buf.len());
                    if buf.is_empty() {
                        should_break = true
                    }
                } else {
                    should_break = true;
                }
                tx.send(buf).map(move |tx| {
                    if should_break {
                        future::Loop::Break(tx)
                    } else {
                        future::Loop::Continue(tx)
                    }
                })
            })
            .then(|_| Ok(())),
        );
        ctx.wait(
            actix::fut::wrap_future(
                ipfs::add(
                    rx.then(|x| x.expect("mpsc unwrap panicked, but never should"))
                        .filter(|x| x.is_empty()),
                    None,
                )
                .and_then(|add_response| ipfs::block_get(add_response.hash.0))
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

    fn stopped(&mut self, _ctx: &mut <Clean as Actor>::Context) {
        match &self.raw_block_data {
            Some(Ok(raw_block_data)) => io::stdout()
                .write_all(raw_block_data)
                .expect("unable to write to stdout"),
            Some(Err(err)) => panic!("{:?}", err),
            _ => panic!("clean stopped before completion"),
        }
    }
}
