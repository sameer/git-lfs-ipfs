use cid::Cid;
use futures::{future, prelude::*};
use lazy_static::lazy_static;
use url::Url;

use crate::error::Error;

lazy_static! {
    static ref IPFS_PUBLIC_API_URL: Url = Url::parse("https://ipfs.io/").unwrap();
}

pub fn sha256_to_cid(
    codec: cid::Codec,
    sha256_str: &str,
) -> impl Future<Item = Cid, Error = Error> {
    future::result(
        hex::decode(sha256_str)
            .ok()
            .and_then(|digest| {
                if digest.len() != 32 {
                    None
                } else {
                    let mut mh = [0u8; 34];
                    mh[0] = multihash::Hash::SHA2256.code();
                    mh[1] = multihash::Hash::SHA2256.size();
                    digest.iter().enumerate().for_each(|(i, x)| mh[i + 2] = *x);
                    Some(Cid::new(codec, cid::Version::V0, &mh))
                }
            })
            .ok_or(Error::HashError),
    )
}
