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
struct Event(custom::Event);

impl Message for Event {
    type Result = Result<Option<Event>, CliError>;
}

#[derive(Fail, Debug)]
enum CliError {
    #[fail(display = "{}", _0)]
    SerdeJsonError(#[cause] serde_json::error::Error),
    #[fail(display = "{}", _0)]
    Io(#[cause] std::io::Error),
    #[fail(display = "Input was an unexpected event {:?}", _0)]
    UnexpectedEvent(Event),
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
    fn started(&mut self, ctx: &mut Context<Self>) {
        let mut read_it =
            std::io::BufReader::new(std::io::stdin())
                .lines()
                .map(|r| -> Result<Event, CliError> {
                    r.map_err(CliError::Io).and_then(|buf| {
                        serde_json::from_str(&buf).map_err(CliError::SerdeJsonError)
                    })
                });

        ctx.add_stream(
            stream::poll_fn(move || -> Poll<Option<Event>, CliError> {
                read_it.next().transpose().map(|x| Async::Ready(x))
            })
            .chain(stream::once(Ok(Event(custom::Event::Terminate)))),
        );
    }
}

impl StreamHandler<Event, CliError> for Communicator {
    fn handle(&mut self, event: Event, ctx: &mut Context<Self>) {
        match (self.engine.clone(), event) {
            (None, Event(custom::Event::Init(init))) => {
                self.engine = Some(Engine::new(init).start());
                println!("{{}}");
            }
            (None, event) => {
                panic!(CliError::UnexpectedEvent(event));
            }
            (Some(_), Event(custom::Event::Init(init))) => {
                panic!(CliError::UnexpectedEvent(Event(custom::Event::Init(init))));
            }
            (Some(_), Event(custom::Event::Terminate)) => {
                debug!("Stopping system");
                System::current().stop();
            }
            (Some(engine), event) => {
                debug!("Sending event {:?}", event);
                ctx.wait(actix::fut::wrap_future(engine.send(event.clone())).then(
                    move |res, actor: &mut Self, ctx| match res.unwrap() {
                        Ok(received_event) => {
                            debug!("Received event {:?}", received_event);
                            if let Some(event) = received_event {
                                println!(
                                    "{}",
                                    serde_json::to_string(&event)
                                        .expect("Failed to serialize an event")
                                );
                            } else {
                                debug!("Replay requested");
                                actor.handle(event, ctx);
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

struct Engine {
    init: custom::Init,
    contents: Option<spec::ipfs::LsResponse>,
}

impl Engine {
    fn new(init: custom::Init) -> Self {
        Self {
            init,
            contents: None,
        }
    }
}

impl Actor for Engine {
    type Context = Context<Self>;
}

impl Handler<Event> for Engine {
    type Result = ResponseActFuture<Self, Option<Event>, CliError>;
    fn handle(&mut self, event: Event, ctx: &mut Context<Self>) -> Self::Result {
        match (event.0, &self.init.operation, self.contents.clone()) {
            (custom::Event::Download(_), custom::Operation::Download, None) => {
                ctx.wait(
                    actix::fut::wrap_future(ipfs::ls(
                        spec::ipfs::Path::from_str(
                            "/ipfs/QmWnE5vczyRHW7CtiRwsXPaQ5BRSbdZh8pAtr3bWGD6SUD",
                        )
                        .unwrap(),
                    ))
                    .then(move |r, actor: &mut Self, _ctx| {
                        actor.contents = r.ok();
                        actix::fut::ok(())
                    }),
                );
                Box::new(actix::fut::wrap_future::<_, Self>(future::ok(None)))
            }
            (custom::Event::Download(download), custom::Operation::Download, Some(contents)) => {
                let link = contents
                    .objects
                    .iter()
                    .flat_map(|x| x.links.iter())
                    .find(|x| x.name == download.object.oid);
                if let Some(link) = link {
                    let mut output = std::env::current_dir().unwrap();
                    output.push(&download.object.oid);
                    Box::new(actix::fut::wrap_future::<_, Self>(
                        ipfs::cat_to_fs(link.clone().into(), output.clone())
                            .map_err(CliError::IpfsApiError)
                            .map(move |_| {
                                Event(custom::Event::Complete(custom::Complete {
                                    oid: download.object.oid.clone(),
                                    error: None,
                                    path: Some(output),
                                }))
                            })
                            .map(|x| Some(x)),
                    ))
                } else {
                    Box::new(actix::fut::wrap_future::<_, Self>(future::ok(Some(Event(
                        custom::Event::Complete(custom::Complete {
                            oid: download.object.oid.clone(),
                            error: Some(custom::Error {
                                code: 404,
                                message: "Object not found".to_string(),
                            }),
                            path: None,
                        }),
                    )))))
                }
            }
            (custom::Event::Upload(upload), custom::Operation::Upload, None) => {
                Box::new(actix::fut::wrap_future::<_, Self>(future::ok(None)))
            }
            (event, _, _) => Box::new(actix::fut::wrap_future::<_, Self>(future::err(
                CliError::UnexpectedEvent(Event(event)),
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
