use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use ipfs_api::IpfsApi;
use std::io::Write;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use git_lfs_spec::transfer::custom::{self, Complete, Error, Event, Operation, Progress};

pub fn read_events(
    input: impl AsyncBufRead + AsyncBufReadExt + Unpin,
) -> impl Stream<Item = Result<Event>> {
    async_stream::stream! {
        let mut lines = input.lines();
        while let Some(line) = lines.next_line().await? {
            let parsed = serde_json::from_str(&line).context("could not parse JSON");
            yield parsed
        }
    }
}

const INTERNAL_SERVER_ERROR: i32 = 500;

pub fn transfer<E: 'static + Send + Sync + std::error::Error>(
    client: impl IpfsApi<Error = E>,
    input_event_stream: impl Stream<Item = Result<Event>>,
) -> impl Stream<Item = Result<Event>> {
    let mut init_opt = None;
    async_stream::stream! {
        futures_util::pin_mut!(input_event_stream);
        while let Some(event) = input_event_stream.next().await.transpose()? {
            match (init_opt.as_ref(), event) {
                (None, Event::Init(init)) => {
                    init_opt = Some(init);
                    yield Ok(Event::AcknowledgeInit)
                }
                (None, event) => {
                    yield Err(anyhow::anyhow!("Unexpected event: {:?}", event))
                }
                (Some(_), Event::Init(init)) => {
                    yield Err(anyhow::anyhow!("Unexpected init event: {:?}", init))
                }

                (Some(_), Event::Terminate) => {
                    break
                }
                (Some(init), event) => {
                    match (event, &init.operation) {
                        (Event::Download(download), Operation::Download) => {
                            let cid_result = crate::ipfs::sha256_to_cid(&download.object.oid);
                            match cid_result {
                                Ok(cid) => {
                                    let oid = download.object.oid.clone();
                                    let mut output_path = std::env::current_dir()?;
                                    output_path.push(&download.object.oid);
                                    let mut output = std::fs::File::create(&output_path)?;

                                    let mut stream =
                                        client.block_get(&format!("/ipfs/{}", cid));
                                    let mut bytes_so_far = 0;
                                    while let Some(res) = stream.next().await {
                                        let bytes = res?;
                                        output.write_all(&bytes)?;
                                        bytes_so_far += bytes.len() as u64;
                                            yield Ok(Event::Progress(
                                                Progress {
                                                    oid: oid.clone(),
                                                    bytes_so_far,
                                                    bytes_since_last: bytes.len() as u64,
                                                }
                                                .into()
                                            ));
                                    }
                                    yield Ok(Event::Complete(
                                        Complete {
                                            oid: download.object.oid.clone(),
                                            result: Some(custom::Result::Path(output_path)),
                                        }
                                        .into(),
                                    ));
                                }
                                Err(err) => {
                                    yield Ok(Event::Complete(
                                        Complete {
                                            oid: download.object.oid.clone(),
                                            result: Some(custom::Result::Error(Error {
                                                code: INTERNAL_SERVER_ERROR,
                                                message: err.to_string(),
                                            })),
                                        }
                                        .into(),
                                    ))
                                },
                            }
                        }
                        // Upload transfer is dummy, clean adds files to IPFS already
                        // TODO: just check the sha256 hash with a /api/v0/block/get
                        (Event::Upload(upload), Operation::Upload) => { yield Ok(Event::Complete(
                                Complete {
                                    oid: upload.object.oid,
                                    result: None,
                                }
                                .into(),
                            ))
                        },
                        (event, _) => {yield Err(anyhow::anyhow!("Unexpected event: {:?}", event))},
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git_lfs_spec::transfer::custom::{Event, Init};
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn read_events_parses_event_successfully() {
        let input: &[u8] = br#"{"event":"init","operation":"download","remote":"origin","concurrent":true,"concurrenttransfers":3}"#;
        let stream = read_events(input);
        futures::pin_mut!(stream);
        let mut events = vec![];
        while let Some(output) = stream.next().await {
            events.push(output.unwrap());
        }
        assert_eq!(
            events,
            &[Event::Init(Init {
                operation: Operation::Download,
                remote: "origin".to_string(),
                concurrent: true,
                concurrenttransfers: Some(3),
            })]
        );
    }
}
