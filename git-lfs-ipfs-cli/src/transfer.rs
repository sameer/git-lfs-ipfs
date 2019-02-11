use std::io::{BufRead, Write};

use actix::prelude::*;
use futures::{future, prelude::*, stream};

use crate::error::CliError;
use git_lfs_ipfs_lib::{
    ipfs,
    spec::{self, transfer::custom},
};

#[derive(Debug, Clone)]
struct Input(custom::Event);

impl Message for Input {
    type Result = Result<Output, CliError>;
}

#[derive(Debug, Clone)]
struct Output(custom::Event);

impl Message for Output {
    type Result = Result<(), ()>;
}

pub struct Transfer {
    engine: Option<actix::Addr<Engine>>,
}

impl Default for Transfer {
    fn default() -> Self {
        Transfer { engine: None }
    }
}

impl Actor for Transfer {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut <Self as Actor>::Context) {
        let mut read_it =
            std::io::BufReader::new(std::io::stdin())
                .lines()
                .map(|r| -> Result<Input, CliError> {
                    r.map_err(CliError::Io).and_then(|buf| {
                        serde_json::from_str(&buf)
                            .map(Input)
                            .map_err(CliError::SerdeJsonError)
                    })
                });
        // TODO: Convert to stdin-reading style used in Clean implementation.
        ctx.add_stream(stream::poll_fn(move || -> Poll<Option<Input>, CliError> {
            read_it.next().transpose().map(Async::Ready)
        }));
    }
}

impl StreamHandler<Input, CliError> for Transfer {
    fn handle(&mut self, event: Input, ctx: &mut <Self as Actor>::Context) {
        match (self.engine.clone(), event) {
            (None, Input(custom::Event::Init(init))) => {
                self.engine = Some(Engine::new(init).start());
                println!("{{}}");
            }
            (None, event) => {
                panic!(CliError::UnexpectedEvent(event.0));
            }
            (Some(_), Input(custom::Event::Init(init))) => {
                panic!(CliError::UnexpectedEvent(custom::Event::Init(init)));
            }
            (Some(_), Input(custom::Event::Terminate)) => {
                debug!("Stopping system");
                System::current().stop();
            }
            (Some(engine), event) => {
                debug!("Sending event {:?}", event);
                ctx.wait(actix::fut::wrap_future(engine.send(event.clone())).then(
                    move |res, actor: &mut Self, ctx| match res.unwrap() {
                        Ok(response) => {
                            debug!("Received response {:?}", response);
                            println!(
                                "{}",
                                serde_json::to_string(&response.0s)
                                    .expect("Failed to serialize an event")
                            );
                            actix::fut::ok(())
                        }
                        Err(err) => {
                            panic!("{:?}", err);
                            actix::fut::err(())
                        }
                    },
                ));
            }
        };
    }

    fn error(&mut self, err: CliError, ctx: &mut Context<Self>) -> Running {
        panic!("{:?}", err);
    }
}

impl Handler<Output> for Transfer {
    type Result = <Output as Message>::Result;

    fn handle(&mut self, event: Output, ctx: &mut <Self as Actor>::Context) -> Self::Result {
        println!(
            "{}",
            serde_json::to_string(&event.0).expect("Failed to serialize an event")
        );
        Ok(())
    }
}

struct Engine {
    init: custom::Init,
}

impl Engine {
    fn new(init: custom::Init) -> Self {
        Self { init }
    }
}

impl Actor for Engine {
    type Context = Context<Self>;
}

impl Handler<Input> for Engine {
    type Result = ResponseActFuture<Self, Output, CliError>;
    fn handle(&mut self, event: Input, ctx: &mut <Self as Actor>::Context) -> Self::Result {
        match (event.0, &self.init.operation) {
            (custom::Event::Download(download), custom::Operation::Download) => {
                let cid = ipfs::sha256_to_cid(cid::Codec::DagProtobuf, &download.object.oid)
                    .wait()
                    .ok();
                if let Some(cid) = cid {
                    let oid = download.object.oid.clone();
                    let mut output_path = std::env::current_dir().unwrap();
                    output_path.push(&download.object.oid);
                    let mut output = std::fs::File::create(&output_path).unwrap();

                    Box::new(
                        actix::fut::wrap_stream(
                            ipfs_api::IpfsClient::default()
                                .block_get(&spec::ipfs::Path::ipfs(cid.clone()).to_string())
                                .map_err(CliError::IpfsApiError)
                                .and_then(move |x| {
                                    output.write_all(&x).map(|_| x.len()).map_err(CliError::Io)
                                }),
                        )
                        .fold(0, move |mut bytes_so_far, x, actor: &mut Self, ctx| {
                            bytes_so_far += x as u64;
                            println!(
                                "{}",
                                serde_json::to_string(&custom::Event::Progress(
                                    custom::Progress {
                                        oid: oid.clone(),
                                        bytes_so_far,
                                        bytes_since_last: x as u64,
                                    }
                                    .into()
                                ))
                                .expect("Failed to serialize an event")
                            );
                            // TODO: Don't disobey actix style and just print events here, there must be a better way...
                            // ctx.spawn(actix::fut::wrap_future(actor.transfer.send(
                            //     Output(custom::Event::Progress(custom::Progress {
                            //         oid: oid.clone(),
                            //         bytes_so_far,
                            //         bytes_since_last: x,
                            //     })),
                            // ).then(|_| {
                            //     future::ok(())
                            // })));
                            actix::fut::ok(bytes_so_far)
                        })
                        .map(move |_, _, _| {
                            Output(custom::Event::Complete(
                                custom::Complete {
                                    oid: download.object.oid.clone(),
                                    error: None,
                                    path: Some(output_path),
                                }
                                .into(),
                            ))
                        }),
                    )
                } else {
                    Box::new(actix::fut::wrap_future::<_, Self>(future::ok(Output(
                        custom::Event::Complete(
                            custom::Complete {
                                oid: download.object.oid.clone(),
                                error: Some(custom::Error {
                                    code: 404,
                                    message: "Object not found".to_string(),
                                }),
                                path: None,
                            }
                            .into(),
                        ),
                    ))))
                }
            }
            // Upload transfer is dummy, the smudge filter adds files to IPFS already
            // TODO: just check the sha256 hash with a /api/v0/block/get
            (custom::Event::Upload(upload), custom::Operation::Upload) => Box::new(
                actix::fut::wrap_future::<_, Self>(future::ok(Output(custom::Event::Complete(
                    custom::Complete {
                        oid: upload.object.oid,
                        error: None,
                        path: None,
                    }
                    .into(),
                )))),
            ),
            (event, _) => Box::new(actix::fut::wrap_future::<_, Self>(future::err(
                CliError::UnexpectedEvent(event),
            ))),
        }
    }
}
