use anyhow::Result;
use futures::StreamExt;
use git_lfs_spec::transfer::custom::Event;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::io::{stdin, stdout, BufReader};

use crate::{clean::clean, smudge::smudge};

mod clean;
mod smudge;
mod transfer;

mod ipfs;

#[derive(Debug, StructOpt)]
#[structopt(author, about)]
enum GitLfsIpfs {
    /// git-lfs smudge filter extension for IPFS
    Smudge {
        /// Name of the file
        filename: PathBuf,
    },
    /// git-lfs clean filter extension for IPFS
    Clean {
        /// Name of the file
        filename: PathBuf,
    },
    /// git-lfs custom transfer for IPFS
    Transfer,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = crate::ipfs::client();
    match GitLfsIpfs::from_args() {
        GitLfsIpfs::Smudge { filename: _ } => smudge(client, stdin(), stdout()).await,
        GitLfsIpfs::Clean { filename: _ } => clean(client, std::io::stdin(), stdout()).await,
        GitLfsIpfs::Transfer => {
            let buffered_stdin = BufReader::new(stdin());
            let input_event_stream = transfer::read_events(buffered_stdin);
            let output_event_stream = transfer::transfer(client, input_event_stream);
            futures_util::pin_mut!(output_event_stream);
            while let Some(output_event) = output_event_stream.next().await.transpose()? {
                if Event::AcknowledgeInit == output_event {
                    println!("{{ }}");
                } else {
                    println!(
                        "{}",
                        serde_json::to_string(&output_event).expect("Failed to serialize an event")
                    );
                }
            }
            Ok(())
        }
    }
}
