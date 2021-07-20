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
                                                    oid: download.object.oid.clone(),
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
                                    oid: upload.object.oid.clone(),
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
    use std::{fs::File, io::Read};

    use super::*;
    use crate::ipfs::client;
    use git_lfs_spec::{
        transfer::custom::{Download, Event, Init, Result, Upload},
        Object,
    };
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    const FILE: &[u8] = b"hello world";
    const OID: &str = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    const SIZE: u64 = FILE.len() as u64;

    #[tokio::test]
    async fn read_events_parses_event_successfully() {
        let init = Event::Init(Init {
            operation: Operation::Download,
            remote: "origin".to_string(),
            concurrent: true,
            concurrenttransfers: Some(3),
        });
        let input: &[u8] = br#"{"event":"init","operation":"download","remote":"origin","concurrent":true,"concurrenttransfers":3}"#;
        let stream = read_events(input);
        futures::pin_mut!(stream);
        let mut events = vec![];
        while let Some(output) = stream.next().await {
            events.push(output.unwrap());
        }
        assert_eq!(events, &[init]);
    }

    #[tokio::test]
    #[ignore]
    async fn transfer_handles_events_as_expected_for_download() {
        let temp_dir = tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let expected_output_path = temp_dir.path().join(&OID);

        let client = client();
        let input_events = [
            Event::Init(Init {
                operation: Operation::Download,
                remote: "origin".to_string(),
                concurrent: true,
                concurrenttransfers: Some(3),
            }),
            Event::Download(
                Download {
                    object: Object {
                        oid: OID.to_string(),
                        size: SIZE,
                    },
                }
                .into(),
            ),
            Event::Terminate,
        ];
        let expected_output_events = [
            Event::AcknowledgeInit,
            Event::Progress(
                Progress {
                    oid: OID.to_string(),
                    bytes_so_far: SIZE,
                    bytes_since_last: SIZE,
                }
                .into(),
            ),
            Event::Complete(
                Complete {
                    oid: OID.to_string(),
                    result: Some(Result::Path(expected_output_path.clone())),
                }
                .into(),
            ),
        ];
        let output_stream = transfer(
            client,
            futures::stream::iter(input_events.iter().cloned().map(anyhow::Result::Ok)),
        );
        futures_util::pin_mut!(output_stream);
        let expected_output_stream = futures::stream::iter(expected_output_events.iter().cloned());
        let mut actual_and_expected_output_stream = output_stream.zip(expected_output_stream);
        while let Some((actual, expected)) = actual_and_expected_output_stream.next().await {
            assert_eq!(actual.unwrap(), expected);
        }

        let mut actual_file = Vec::with_capacity(FILE.len());

        File::open(&expected_output_path)
            .unwrap()
            .read_to_end(&mut actual_file)
            .unwrap();

        assert_eq!(actual_file, FILE)
    }

    #[tokio::test]
    #[ignore]
    async fn transfer_handles_events_as_expected_for_upload() {
        let temp_dir = tempdir().unwrap();
        let temp_file = temp_dir.path().join(OID);
        std::fs::File::create(&temp_file).unwrap();

        let client = client();
        let input_events = [
            Event::Init(Init {
                operation: Operation::Upload,
                remote: "origin".to_string(),
                concurrent: true,
                concurrenttransfers: Some(3),
            }),
            Event::Upload(
                Upload {
                    object: Object {
                        oid: OID.to_string(),
                        size: SIZE,
                    },
                    path: temp_file.clone(),
                }
                .into(),
            ),
            Event::Terminate,
        ];
        let expected_output_events = [
            Event::AcknowledgeInit,
            Event::Complete(
                Complete {
                    oid: OID.to_string(),
                    result: None,
                }
                .into(),
            ),
        ];
        let output_stream = transfer(
            client,
            futures::stream::iter(input_events.iter().cloned().map(anyhow::Result::Ok)),
        );
        futures_util::pin_mut!(output_stream);
        let expected_output_stream = futures::stream::iter(expected_output_events.iter().cloned());
        let mut actual_and_expected_output_stream = output_stream.zip(expected_output_stream);
        while let Some((actual, expected)) = actual_and_expected_output_stream.next().await {
            assert_eq!(actual.unwrap(), expected);
        }

        std::fs::remove_file(temp_file).unwrap();
    }
}
