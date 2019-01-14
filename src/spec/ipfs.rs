use lazy_static::lazy_static;
use serde_derive::Deserialize;

use std::fmt::Display;

pub const EMPTY_FOLDER_HASH: &str = "QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddResponse {
    pub name: String,
    pub hash: String,
    pub size: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KeyListResponse {
    pub keys: Vec<Key>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Key {
    pub name: String,
    pub id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LsResponse {
    pub objects: Vec<Object>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Object {
    pub hash: String,
    pub links: Vec<Link>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectResponse {
    #[serde(flatten)]
    pub object: Object,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Link {
    pub name: String,
    pub hash: String,
    pub size: String,
    pub Type: String, // Not sure how to handle this
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Prefix {
    Ipfs,
    Ipns,
}

impl Prefix {
    pub fn is_dnslink_allowed(&self) -> bool {
        match *self {
            Prefix::Ipns => true,
            _ => false,
        }
    }
}

impl Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Prefix::Ipns => write!(f, "ipns"),
            Prefix::Ipfs => write!(f, "ipfs"),
        }
    }
}

pub enum PathType {
    Cid(cid::Cid),
    DnsLink(publicsuffix::Domain),
}

impl Display for PathType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PathType::Cid(cid) => write!(f, "{}", cid),
            PathType::DnsLink(domain) => write!(f, "{}", domain),
        }
    }
}

lazy_static! {
    static ref public_suffix_list: publicsuffix::List = publicsuffix::List::fetch().unwrap();
}

impl PathType {
    pub fn parse(input: &str) -> Option<Self> {
        use cid::ToCid;
        if let Ok(cid) = input.to_cid() {
            Some(PathType::Cid(cid))
        } else if let Ok(dns_link) = public_suffix_list.parse_domain(input) {
            Some(PathType::DnsLink(dns_link))
        } else {
            None
        }
    }
}

pub struct IpfsPath {
    pub prefix: Prefix,
    pub path_type: PathType,
}

impl IpfsPath {
    pub fn parse(prefix: Prefix, path_type: PathType) -> Option<Self> {
        if prefix.is_dnslink_allowed() {
            Self { prefix, path_type }.into()
        } else if let PathType::Cid(cid) = path_type {
            Self {
                prefix,
                path_type: PathType::Cid(cid),
            }
            .into()
        } else {
            None
        }
    }
}

impl Display for IpfsPath {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "/{}/{}", self.prefix, self.path_type)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn ipfs_path_correct() {
        assert_eq!(
            EMPTY_FOLDER_HASH,
            format!("{}", PathType::parse(EMPTY_FOLDER_HASH).unwrap())
        );
        assert_eq!(
            "ipfs.io",
            format!("{}", PathType::parse("ipfs.io").unwrap())
        );
        let path_string = format!("/ipfs/{}", EMPTY_FOLDER_HASH);
        assert_eq!(
            path_string,
            format!(
                "{}",
                IpfsPath::parse(Prefix::Ipfs, PathType::parse(EMPTY_FOLDER_HASH).unwrap()).unwrap()
            )
        );
    }
}
