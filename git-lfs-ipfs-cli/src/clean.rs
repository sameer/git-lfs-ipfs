use anyhow::Result;
use futures::StreamExt;
use ipfs_api::{response::AddResponse, IpfsApi};
use std::io::Read;
use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Replace file contents with the raw IPFS block contents.
///
/// This means two things:
/// 1. The file must be added to IPFS during clean.
/// 1. The SHA-256 hash stored by git-lfs will be
///    identical to the Qmhash, allowing retrieval of the
///    file's contents via IPFS.
///
/// <https://github.com/git-lfs/git-lfs/blob/main/docs/extensions.md#clean>
pub async fn clean<E: 'static + Send + Sync + std::error::Error>(
    client: impl IpfsApi<Error = E>,
    input: impl Read + Send + Sync + 'static,
    mut output: impl AsyncWrite + AsyncWriteExt + Unpin,
) -> Result<()> {
    let AddResponse { hash, .. } = client.add(input).await?;
    let mut stream = client.block_get(&hash);
    while let Some(bytes) = stream.next().await.transpose()? {
        output.write_all(&bytes).await?;
    }

    Ok(())
}
