use anyhow::Result;
use hyper::client::HttpConnector;
use ipfs_api::IpfsClient;
use multihash::{Code, MultihashDigest, Sha2Digest, U32};

/// Assuming that the sha256 hash is for a Qmhash
pub fn sha256_to_cid(sha256_str: &str) -> Result<cid::Cid> {
    let raw_digest = hex::decode(sha256_str)?;
    if raw_digest.len() != 32 {
        Err(anyhow::anyhow!(
            "SHA256 digest should have 32 bytes but had {}",
            raw_digest.len()
        ))
    } else {
        let mut digest = Sha2Digest::<U32>::default();
        for i in 0..32 {
            digest.as_mut()[i] = raw_digest[i];
        }
        Ok(cid::Cid::new_v0(Code::multihash_from_digest(&digest))?)
    }
}

pub fn client() -> IpfsClient<hyper_rustls::HttpsConnector<HttpConnector>> {
    IpfsClient::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const _INPUT: &str = "hello world";
    const HASH_SUM: &str = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    const QM_HASH_SUM: &str = "QmaozNR7DZHQK1ZcU9p7QdrshMvXqWK6gpu5rmrkPdT3L4";

    #[test]
    fn sha256_to_cid_returns_cid_for_valid_hash() {
        assert_eq!(sha256_to_cid(HASH_SUM).unwrap().to_string(), QM_HASH_SUM);
    }

    #[test]
    fn sha256_to_cid_returns_err_for_string_without_32_bytes() {
        assert!(sha256_to_cid("abcd").is_err());
    }

    #[test]
    fn sha256_to_cid_returns_err_for_non_hex_string() {
        assert!(sha256_to_cid("foo").is_err());
    }
}
