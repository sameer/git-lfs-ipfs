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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipfs::client;
    use std::io::Cursor;

    const FILE: &[u8] = b"hello world";
    const RAW_BLOCK: &[u8] = include_bytes!("../test/hello_world_raw_block");

    #[tokio::test]
    #[ignore]
    async fn clean_converts_file_into_raw_root_block() {
        let client = client();
        let mut cursor = Cursor::new(vec![]);
        clean(client, FILE, &mut cursor).await.unwrap();
        assert_eq!(&cursor.into_inner(), RAW_BLOCK);
    }
}
