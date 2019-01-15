use cid::Cid;
use lazy_static::lazy_static;
use serde_derive::{Deserialize, Serialize};

use std::fmt::Display;
use std::path::PathBuf;

pub const EMPTY_FOLDER_HASH: &str = "QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
#[serde(untagged)]
pub enum Result<T> {
    Ok(T),
    Err(Error),
}

impl<T> Into<std::result::Result<T, Error>> for Result<T> {
    fn into(self) -> std::result::Result<T, Error> {
        match self {
            Result::Ok(t) => Ok(t),
            Result::Err(err) => Err(err),
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Error {
    message: String,
    code: u64,
    Type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddResponse {
    pub name: String,
    #[serde(with = "string")]
    pub hash: Cid,
    pub size: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KeyListResponse {
    pub keys: Vec<Key>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Key {
    pub name: String,
    #[serde(with = "string")]
    pub id: Cid,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LsResponse {
    pub objects: Vec<Object>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Object {
    #[serde(with = "string")]
    pub hash: Cid,
    pub links: Vec<Link>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", transparent)]
pub struct ObjectResponse {
    pub object: Object,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Link {
    pub name: String,
    #[serde(with = "string")]
    pub hash: Cid,
    pub size: u64,
    pub Type: i32, // Not sure how to handle this
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct CidResponse {
    #[serde(with = "string")]
    pub cid: Cid,
}

// #[derive(Deserialize)]
// #[serde(rename_all = "PascalCase")]
// pub enum PathResponse {
//     Success{
//         path: Path,
//     },
//     Error(Error),
// }

#[derive(Deserialize, PartialEq, Eq, Clone)]
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

#[derive(PartialEq, Eq)]
pub enum Root {
    Cid(cid::Cid),
    DnsLink(publicsuffix::Domain),
}

impl Display for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Root::Cid(cid) => write!(f, "{}", cid),
            Root::DnsLink(domain) => write!(f, "{}", domain),
        }
    }
}

lazy_static! {
    static ref public_suffix_list: publicsuffix::List = publicsuffix::List::fetch().unwrap();
}

impl Root {
    pub fn parse(input: &str) -> Option<Self> {
        use cid::ToCid;
        if let Ok(cid) = input.to_cid() {
            Some(Root::Cid(cid))
        } else if let Ok(dns_link) = public_suffix_list.parse_domain(input) {
            Some(Root::DnsLink(dns_link))
        } else {
            None
        }
    }
}

pub struct Path {
    pub prefix: Prefix,
    pub root: Root,
    pub suffix: Option<PathBuf>,
}

impl Path {
    pub fn parse(prefix: Prefix, root: Root, suffix: Option<PathBuf>) -> Option<Self> {
        if prefix.is_dnslink_allowed() {
            Self {
                prefix,
                root,
                suffix,
            }
            .into()
        } else if let Root::Cid(cid) = root {
            Self {
                prefix,
                root: Root::Cid(cid),
                suffix,
            }
            .into()
        } else {
            None
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::path::PathBuf;
        let mut buf = PathBuf::from("/");
        buf.push(self.prefix.to_string());
        buf.push(self.root.to_string());
        if let Some(suffix) = &self.suffix {
            buf.push(suffix);
        }
        write!(f, "{}", buf.display())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn ipfs_path_correct() {
        assert_eq!(
            EMPTY_FOLDER_HASH,
            format!("{}", Root::parse(EMPTY_FOLDER_HASH).unwrap())
        );
        assert_eq!("ipfs.io", format!("{}", Root::parse("ipfs.io").unwrap()));
        let path_string = format!("/ipfs/{}", EMPTY_FOLDER_HASH);
        assert_eq!(
            path_string,
            format!(
                "{}",
                Path::parse(Prefix::Ipfs, Root::parse(EMPTY_FOLDER_HASH).unwrap(), None).unwrap()
            )
        );
    }
}

mod string {
    use std::fmt::Display;

    use cid::Cid;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Cid, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}
