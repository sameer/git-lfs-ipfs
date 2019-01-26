use cid::Cid;
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_derive::{Deserialize, Serialize};

use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::str::FromStr;

pub const EMPTY_FOLDER_HASH: &str = "QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn";

lazy_static! {
    pub static ref EMPTY_FOLDER_PATH: Path = Path::from_str("/ipfs/QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn").unwrap();
}

#[derive(Deserialize)]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LsResponse {
    pub objects: Vec<ObjectPath>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectCid {
    #[serde(with = "string")]
    pub hash: Cid,
    pub links: Vec<Link>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectPath {
    #[serde(with = "string")]
    pub hash: Path,
    pub links: Vec<Link>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectResponse {
    #[serde(with = "string")]
    pub hash: Cid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Link {
    pub name: String,
    #[serde(with = "string")]
    pub hash: Cid,
    pub size: u64,
    pub Type: i32, // Not sure how to handle this
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResolveResponse {
    #[serde(with = "string")]
    pub path: Path,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Prefix {
    Ipfs,
    Ipns,
}

impl FromStr for Prefix {
    type Err = crate::error::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ipfs" => Ok(Prefix::Ipfs),
            "ipns" => Ok(Prefix::Ipns),
            _ => Err(crate::error::Error::IpfsPathParseError(
                "Prefix was neither ipfs nor ipns",
            )),
        }
    }
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

impl FromStr for Root {
    type Err = crate::error::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use cid::ToCid;
        if let Ok(cid) = s.to_cid() {
            Ok(Root::Cid(cid))
        } else if let Ok(dns_link) = public_suffix_list.parse_domain(s) {
            Ok(Root::DnsLink(dns_link))
        } else {
            Err(crate::error::Error::IpfsPathParseError(
                "Root was neither a CID nor DNS record",
            ))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
        write!(f, "/{}/{}", self.prefix, self.root).and_then(|x| {
            if let Some(suffix) = &self.suffix {
                write!(f, "/{}", suffix.display())
            } else {
                Ok(x)
            }
        })
    }
}

impl FromStr for Path {
    type Err = crate::error::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut it = s.split('/').into_iter();
        it.next();
        let prefix = it.next();
        let root = it.next();
        let suffix: Option<String> =
            Some(it.collect()).and_then(|s: String| if s.len() != 0 { Some(s) } else { None });
        if let (Some(prefix), Some(root)) = (prefix, root) {
            if let (Ok(prefix), Ok(root)) = (Prefix::from_str(prefix), Root::from_str(root)) {
                Self::parse(
                    prefix,
                    root,
                    suffix.and_then(|suffix| PathBuf::from_str(&suffix).ok()),
                )
                .ok_or(crate::error::Error::IpfsPathParseError("Parse failed"))
            } else {
                Err(crate::error::Error::IpfsPathParseError(
                    "Prefix and root failed",
                ))
            }
        } else {
            Err(crate::error::Error::IpfsPathParseError(
                "No prefix or root possible",
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn ipfs_path_correct() {
        assert_eq!(
            EMPTY_FOLDER_HASH,
            format!("{}", Root::from_str(EMPTY_FOLDER_HASH).unwrap())
        );
        assert_eq!("ipfs.io", format!("{}", Root::from_str("ipfs.io").unwrap()));
        let path_string = format!("/ipfs/{}", EMPTY_FOLDER_HASH);
        assert_eq!(
            path_string,
            format!("{}", Path::from_str(&path_string).unwrap())
        );
    }
}


// TODO: Refactor to implement serialize for IpfsPath
pub mod string {
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
        T::Err: Display,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}
