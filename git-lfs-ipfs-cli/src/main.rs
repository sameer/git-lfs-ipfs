extern crate actix;
extern crate dirs;
extern crate env_logger;
extern crate failure;
extern crate futures;
extern crate lazy_static;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate url_serde;
#[macro_use]
extern crate log;

extern crate git_lfs_ipfs_lib;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use std::io::BufRead;
use std::str::FromStr;

use actix::prelude::*;
use failure::Fail;
use futures::{future, prelude::*, stream};
use git_lfs_ipfs_lib::*;
use serde_derive::{Deserialize, Serialize};

use spec::transfer::custom;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
struct InputEvent(custom::Event);

impl Message for InputEvent {
    type Result = Result<Response, CliError>;
}

#[derive(Debug, Clone)]
enum Response {
    Replay, // 1st download request waits until ls response is received, then the response for that is sent back, system termination will still require that a response come back later (?)
    // Streaming(Box<dyn Stream<Item = custom::Event, Error = ()>>),
    Now(custom::Event),
}

impl Message for Response {
    type Result = Result<(), ()>;
}

#[derive(Fail, Debug)]
enum CliError {
    #[fail(display = "{}", _0)]
    SerdeJsonError(#[cause] serde_json::error::Error),
    #[fail(display = "{}", _0)]
    Io(#[cause] std::io::Error),
    #[fail(display = "Input was an unexpected event {:?}", _0)]
    UnexpectedEvent(InputEvent),
    #[fail(display = "Error with a request to the IPFS API {:?}", _0)]
    IpfsApiError(error::Error),
}

struct Communicator {
    engine: Option<actix::Addr<Engine>>,
}

impl Default for Communicator {
    fn default() -> Self {
        Self { engine: None }
    }
}

impl Actor for Communicator {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut <Self as Actor>::Context) {
        let mut read_it = std::io::BufReader::new(std::io::stdin()).lines().map(
            |r| -> Result<InputEvent, CliError> {
                r.map_err(CliError::Io)
                    .and_then(|buf| serde_json::from_str(&buf).map_err(CliError::SerdeJsonError))
            },
        );

        ctx.add_stream(stream::poll_fn(
            move || -> Poll<Option<InputEvent>, CliError> {
                read_it.next().transpose().map(|x| Async::Ready(x))
            },
        ));
    }
}

impl StreamHandler<InputEvent, CliError> for Communicator {
    fn handle(&mut self, event: InputEvent, ctx: &mut <Self as Actor>::Context) {
        match (self.engine.clone(), event) {
            (None, InputEvent(custom::Event::Init(init))) => {
                self.engine = Some(Engine::new(ctx.address(), init).start());
                println!("{{}}");
            }
            (None, event) => {
                panic!(CliError::UnexpectedEvent(event));
            }
            (Some(_), InputEvent(custom::Event::Init(init))) => {
                panic!(CliError::UnexpectedEvent(InputEvent(custom::Event::Init(
                    init
                ))));
            }
            (Some(engine), InputEvent(custom::Event::Terminate)) => {
                debug!("Stopping system");
                System::current().stop();
            }
            (Some(engine), event) => {
                debug!("Sending event {:?}", event);
                ctx.wait(actix::fut::wrap_future(engine.send(event.clone())).then(
                    move |res, actor: &mut Self, ctx| match res.unwrap() {
                        Ok(response) => {
                            debug!("Received response {:?}", response);
                            match response {
                                Response::Now(event) => {
                                    println!(
                                        "{}",
                                        serde_json::to_string(&event)
                                            .expect("Failed to serialize an event")
                                    );
                                }
                                Replay => {
                                    StreamHandler::<InputEvent, CliError>::handle(
                                        actor, event, ctx,
                                    );
                                }
                            }
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

impl Handler<Response> for Communicator {
    type Result = <Response as Message>::Result;

    fn handle(&mut self, res: Response, ctx: &mut <Self as Actor>::Context) -> Self::Result {
        match res {
            Response::Now(event) => {
                println!(
                    "{}",
                    serde_json::to_string(&event).expect("Failed to serialize an event")
                );
            }
            _ => {}
        }
        Ok(())
    }
}

struct Engine {
    communicator: actix::Addr<Communicator>,
    init: custom::Init,
    contents: Option<spec::ipfs::LsResponse>,
}

impl Engine {
    fn new(communicator: actix::Addr<Communicator>, init: custom::Init) -> Self {
        Self {
            communicator,
            init,
            contents: None,
        }
    }
}

impl Actor for Engine {
    type Context = Context<Self>;
}

impl Handler<InputEvent> for Engine {
    type Result = ResponseActFuture<Self, Response, CliError>;
    fn handle(&mut self, event: InputEvent, ctx: &mut <Self as Actor>::Context) -> Self::Result {
        match (event.0, &self.init.operation, self.contents.clone()) {
            (custom::Event::Download(download), custom::Operation::Download, None) => {
                ctx.wait(
                    actix::fut::wrap_future(ipfs::ls(
                        spec::ipfs::Path::from_str(
                            "/ipfs/QmWnE5vczyRHW7CtiRwsXPaQ5BRSbdZh8pAtr3bWGD6SUD",
                        )
                        .unwrap(),
                    ))
                    .then(move |r, actor: &mut Self, ctx| {
                        actor.contents = r.ok();
                        actix::fut::ok(())
                    }),
                );
                Box::new(actix::fut::wrap_future::<_, Self>(future::ok(
                    Response::Replay,
                )))
            }
            (custom::Event::Download(download), custom::Operation::Download, Some(contents)) => {
                let link = contents
                    .objects
                    .iter()
                    .flat_map(|x| x.links.iter())
                    .find(|x| x.name == download.object.oid);
                let oid = download.object.oid.clone();
                if let Some(link) = link {
                    let mut output = std::env::current_dir().unwrap();
                    output.push(&download.object.oid);
                    Box::new(
                        actix::fut::wrap_stream(
                            ipfs::cat_to_fs(link.clone().into(), output.clone())
                                .map_err(CliError::IpfsApiError),
                        )
                        .fold(0, move |mut bytes_so_far, x, actor: &mut Self, ctx| {
                            bytes_so_far += x;
                            println!(
                                "{}",
                                serde_json::to_string(&custom::Event::Progress(
                                    custom::Progress {
                                        oid: oid.clone(),
                                        bytes_so_far,
                                        bytes_since_last: x,
                                    }
                                ))
                                .expect("Failed to serialize an event")
                            );
                            // ctx.spawn(actix::fut::wrap_future(actor.communicator.send(
                            //     Response::Now(custom::Event::Progress(custom::Progress {
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
                            Response::Now(custom::Event::Complete(custom::Complete {
                                oid: download.object.oid.clone(),
                                error: None,
                                path: Some(output),
                            }))
                        }),
                    )
                } else {
                    Box::new(actix::fut::wrap_future::<_, Self>(future::ok(
                        Response::Now(custom::Event::Complete(custom::Complete {
                            oid: download.object.oid.clone(),
                            error: Some(custom::Error {
                                code: 404,
                                message: "Object not found".to_string(),
                            }),
                            path: None,
                        })),
                    )))
                }
            }
            // (custom::Event::Upload(upload), custom::Operation::Upload, None) => {
            //     Box::new(actix::fut::wrap_future::<_, Self>(future::ok(None)))
            // }
            (event, _, _) => Box::new(actix::fut::wrap_future::<_, Self>(future::err(
                CliError::UnexpectedEvent(InputEvent(event)),
            ))),
        }
    }
}

fn main() {
    env_logger::init();
    let sys = System::new("git-lfs-ipfs");
    Communicator::default().start();
    sys.run();
}
