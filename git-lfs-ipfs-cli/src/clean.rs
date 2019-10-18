use std::io::{self, Write};

use actix::prelude::*;

use crate::error::CliError;

pub struct Clean {}

impl Default for Clean {
    fn default() -> Self {
        Self {}
    }
}

impl Actor for Clean {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut <Clean as Actor>::Context) {
        ctx.wait(
            actix::fut::wrap_future(
                ipfs_api::IpfsClient::default()
                    .add(io::stdin())
                    .map_err(|err| CliError::IpfsApiError(err.to_string()))
                    .map(|add_response| {
                        ipfs_api::IpfsClient::default()
                            .block_get(&add_response.hash)
                            .map_err(|err| CliError::IpfsApiError(err.to_string()))
                    })
                    .flatten_stream()
                    .fold(io::stdout(), |mut acc, x| {
                        acc.write_all(&x).map_err(CliError::Io)?;
                        Ok(acc)
                    }),
            )
            .then(|result, actor: &mut Self, _ctx| match result {
                Ok(_) => {
                    System::current().stop();
                    actix::fut::ok(())
                }
                Err(err) => panic!("{:?}", err),
            }),
        );
    }

    // fn stopped(&mut self, _ctx: &mut <Clean as Actor>::Context) {
    //     match &self.raw_block_data {
    //         Some(Ok(raw_block_data)) => io::stdout()
    //             .write_all(raw_block_data)
    //             .expect("unable to write to stdout"),
    //         Some(Err(err)) => panic!("{:?}", err),
    //         _ => panic!("clean stopped before completion"),
    //     }
    // }
}
