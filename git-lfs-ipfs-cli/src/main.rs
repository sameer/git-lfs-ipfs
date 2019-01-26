extern crate actix;
extern crate dirs;
extern crate failure;
extern crate futures;
extern crate lazy_static;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate url_serde;

extern crate git_lfs_ipfs_lib;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use std::io::BufRead;

use actix::prelude::*;
use failure::Fail;
use futures::{future, prelude::*, stream};
use git_lfs_ipfs_lib::*;
use serde_derive::{Deserialize, Serialize};

use spec::transfer::custom;

#[derive(Serialize, Deserialize, Debug)]
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
            }
            (None, event) => {
                panic!(CliError::UnexpectedEvent(event));
            }
            (Some(engine), Event(custom::Event::Init(init))) => {
                panic!(CliError::UnexpectedEvent(Event(custom::Event::Init(init))));
            }
            (Some(engine), event) => {
                ctx.wait(actix::fut::wrap_future(engine.send(event).then(
                    |res| match res.unwrap() {
                        Ok(event) => {
                            if let Some(event) = event {
                                println!(
                                    "{}",
                                    serde_json::to_string(&event)
                                        .expect("Failed to serialize an event")
                                );
                            }
                            Ok(())
                        }
                        Err(err) => {
                            panic!("{:?}", err);
                            Err(())
                        }
                    },
                )));
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
        match (event.0, &self.init.operation) {
            (custom::Event::Terminate, _) => {
                System::current().stop();
                Box::new(actix::fut::wrap_future::<_, Self>(future::ok(None)))
            }
            (custom::Event::Download(download), custom::Operation::Download) => {
                Box::new(actix::fut::wrap_future::<_, Self>(future::ok(None)))
            }
            (custom::Event::Upload(upload), custom::Operation::Upload) => {
                Box::new(actix::fut::wrap_future::<_, Self>(future::ok(None)))
            }
            (event, _) => Box::new(actix::fut::wrap_future::<_, Self>(future::err(
                CliError::UnexpectedEvent(Event(event)),
            ))),
        }
    }
}

fn main() {
    let sys = System::new("git-lfs-ipfs");
    Communicator::default().start();
    sys.run();
}
