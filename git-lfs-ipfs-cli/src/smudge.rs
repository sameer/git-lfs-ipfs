use anyhow::Result;
use cid::Cid;
use futures::stream::StreamExt;
use ipfs_api_backend_hyper::IpfsApi;
use multihash::{Code, Hasher, Multihash, MultihashDigest, Sha2_256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Verbatim from IPFS cli docs:
///
/// > Different chunking strategies will produce different
/// hashes for the same file. The default is a fixed block size of
/// 256 * 1024 bytes
const CHUNKER_FIXED_BLOCK_SIZE: usize = 256 * 1024;

const BUFFER_SIZE: usize = CHUNKER_FIXED_BLOCK_SIZE / 256;

async fn sha256_hash_of_raw_block(mut input: impl AsyncRead + Unpin) -> Result<Multihash> {
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut hasher = Sha2_256::default();
    loop {
        let bytes_read = input.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(Code::Sha2_256.wrap(hasher.finalize())?)
}

async fn cid_of_raw_block(input: impl AsyncRead + Unpin) -> Result<Cid> {
    let sha256_hash = sha256_hash_of_raw_block(input).await?;
    Ok(Cid::new_v0(sha256_hash).unwrap())
}

/// Convert a file's raw IPFS block back into the file itself
///
/// Recall that git-lfs is actually storing the QmHash but it
/// wants to get the file's original SHA-256 back.
///
/// <https://github.com/git-lfs/git-lfs/blob/main/docs/extensions.md#smudge>
pub async fn smudge<E: 'static + Send + Sync + std::error::Error>(
    client: impl IpfsApi<Error = E>,
    input: impl AsyncRead + Unpin,
    mut output: impl AsyncWrite + Unpin,
) -> Result<()> {
    let cid = cid_of_raw_block(input).await?;
    let mut stream = client.cat(&format!("/ipfs/{}", cid));
    while let Some(bytes) = stream.next().await.transpose()? {
        output.write_all(&bytes).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::ipfs::client;

    use super::*;
    use pretty_assertions::assert_eq;

    const RAW_BLOCK: &[u8] = include_bytes!("../test/hello_world_raw_block");
    const SHA256_HASH: &str = "f852c7fa62f971817f54d8a80dcd63fcf7098b3cbde9ae8ec1ee449013ec5db0";
    const MULTI_HASH: &str = "Qmf412jQZiuVUtdgnB36FXFX7xg5V6KEbSJ4dpQuhkLyfD";

    #[tokio::test]
    async fn sha256_hash_of_raw_block_returns_expected_hash() {
        assert_eq!(
            sha256_hash_of_raw_block(RAW_BLOCK).await.unwrap().digest(),
            hex::decode(SHA256_HASH).unwrap()
        );
    }

    #[tokio::test]
    async fn cid_of_raw_block_returns_expected_cid() {
        assert_eq!(
            cid_of_raw_block(RAW_BLOCK).await.unwrap().to_string(),
            MULTI_HASH
        );
    }

    #[tokio::test]
    #[ignore]
    async fn smudge_converts_raw_block_into_file_contents() {
        let client = client();
        let mut cursor = Cursor::new(vec![]);
        smudge(client, RAW_BLOCK, &mut cursor).await.unwrap();
        assert_eq!(String::from_utf8_lossy(&cursor.into_inner()), "hello world");
    }
}
