use failure::Fail;
use lazy_static::lazy_static;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_derive::{Deserialize, Serialize};

use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::str::FromStr;

pub const EMPTY_FOLDER_HASH: &str = "QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn";

lazy_static! {
    pub static ref EMPTY_FOLDER_PATH: Path =
        Path::from_str("/ipfs/QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn").unwrap();
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Result<T: Debug + PartialEq> {
    Ok(T),
    Err(Error),
}

#[derive(Deserialize, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Error {
    message: String,
    code: u64,
    Type: String,
}

/// https://docs.ipfs.io/reference/api/http/#api-v0-add
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct AddResponse {
    pub name: String,
    pub hash: Cid,
    pub size: String,
}

/// https://docs.ipfs.io/reference/api/http/#api-v0-key-list
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct KeyListResponse {
    pub keys: Vec<Key>,
}

/// https://docs.ipfs.io/reference/api/http/#api-v0-key-list
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Key {
    pub name: String,
    pub id: Cid,
}

/// https://docs.ipfs.io/reference/api/http/#api-v0-ls
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct LsResponse {
    pub objects: Vec<ObjectPath>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectCid {
    pub hash: Cid,
    pub links: Vec<Link>,
}

/// https://docs.ipfs.io/reference/api/http/#api-v0-ls
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectPath {
    pub hash: Path,
    pub links: Vec<Link>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectResponse {
    pub hash: Cid,
}

/// https://docs.ipfs.io/reference/api/http/#api-v0-object-links
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Link {
    pub name: String,
    pub hash: Cid,
    pub size: u64,
    pub Type: i32, // Not sure how to handle this
}

impl Into<Path> for Link {
    fn into(self) -> Path {
        Path {
            root: Root::Ipfs(self.hash.0),
            suffix: None,
        }
    }
}

/// https://docs.ipfs.io/reference/api/http/#api-v0-resolve
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct ResolveResponse {
    pub path: Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cid(pub cid::Cid);

impl<'de> Deserialize<'de> for Cid {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de;
        use std::fmt;
        struct CidVisitor;
        impl<'de> de::Visitor<'de> for CidVisitor {
            type Value = Cid;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a string that can be converted to a Cid")
            }

            fn visit_str<E>(self, path_str: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                use cid::ToCid;
                path_str.to_cid().map(Cid).map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_string(CidVisitor)
    }
}

#[derive(Fail, Debug, Eq, PartialEq)]
pub enum PathParseError {
    #[fail(display = "unable to parse cid: {}", _0)]
    CidError(cid::Error),
    #[fail(display = "invalid domain: {}", _0)]
    DnsLinkDomainInvalid(String),
    #[fail(display = "errors during UTS#46 processing: {}", _0)]
    DnsLinkUnicodeError(String),
    #[fail(display = "unable to parse suffix: {}", _0)]
    SuffixError(std::string::ParseError),
    #[fail(display = "suffix is not absolute: {}", _0)]
    SuffixNotAbsolute(String),
    #[fail(display = "unexpected prefix: {} (must be /ipfs/ or /ipns/)", _0)]
    UnknownPrefix(String),
    #[fail(display = "expected cid, got dnslink record")]
    ExpectedCid,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Root {
    Ipfs(cid::Cid),
    Ipns(cid::Cid),
    DnsLink(publicsuffix::Domain),
}

impl Display for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Root::Ipfs(cid) => write!(f, "/ipfs/{}", cid),
            Root::Ipns(cid) => write!(f, "/ipns/{}", cid),
            Root::DnsLink(domain) => write!(f, "/ipns/{}", domain),
        }
    }
}

lazy_static! {
    static ref PUBLIC_SUFFIX_LIST: publicsuffix::List = publicsuffix::List::fetch().unwrap();
}

impl FromStr for Root {
    type Err = PathParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use cid::ToCid;
        match (s.get(0..6), s.get(6..)) {
            (Some("/ipfs/"), Some(s)) => {
                s.to_cid().map(Root::Ipfs).map_err(PathParseError::CidError)
            }
            (Some("/ipns/"), Some(s)) => s
                .to_cid()
                .map(Root::Ipns)
                .map_err(PathParseError::CidError)
                .or_else(|_| {
                    PUBLIC_SUFFIX_LIST
                        .parse_domain(s)
                        .map(Root::DnsLink)
                        .map_err(|e| {
                            use publicsuffix::errors::ErrorKind;
                            match e.0 {
                                ErrorKind::Uts46(errs) => {
                                    PathParseError::DnsLinkUnicodeError(format!("{:?}", errs))
                                }
                                ErrorKind::InvalidDomain(domain) => {
                                    PathParseError::DnsLinkDomainInvalid(domain)
                                }
                                _ => panic!("unhandled publicsuffix error"),
                            }
                        })
                }),
            (other, _) => Err(PathParseError::UnknownPrefix(
                other.unwrap_or_default().to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Path {
    pub root: Root,
    pub suffix: Option<PathBuf>,
}

impl Path {
    pub fn ipfs(cid: cid::Cid) -> Self {
        Self {
            root: Root::Ipfs(cid),
            suffix: None,
        }
    }
}

impl FromStr for Path {
    type Err = PathParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use path_clean::PathClean;
        let root_end = s
            .match_indices('/')
            .nth(2)
            .map(|(x, _)| x)
            .unwrap_or_else(|| s.len());
        let root = Root::from_str(s.get(0..root_end).unwrap_or_default())?;
        let suffix = s
            .get(root_end..)
            .and_then(|x| if x.is_empty() { None } else { Some(x) })
            .map(PathBuf::from_str)
            .map(|res| {
                res.map(|x| x.clean())
                    .map_err(PathParseError::SuffixError)
                    .and_then(|x| {
                        if x.is_absolute() {
                            Ok(x)
                        } else {
                            Err(PathParseError::SuffixNotAbsolute(
                                x.to_string_lossy().to_string(),
                            ))
                        }
                    })
            })
            .transpose()
            .map(|x| {
                if let Some(x) = x {
                    if x.parent().is_none() {
                        None
                    } else {
                        Some(x)
                    }
                } else {
                    None
                }
            })?;
        Ok(Self { root, suffix })
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.root)?;
        if let Some(suffix) = &self.suffix {
            write!(f, "{}", suffix.to_string_lossy())
        } else {
            Ok(())
        }
    }
}

impl Into<String> for &Path {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl Serialize for Path {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = self.into();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for Path {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de;
        use std::fmt;
        struct PathVisitor;
        impl<'de> de::Visitor<'de> for PathVisitor {
            type Value = Path;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a string that can be converted to a Path")
            }

            fn visit_str<E>(self, path_str: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Path::from_str(path_str).map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_string(PathVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn root_ipfs_ok() {
        let ipfs_root_str = format!("/ipfs/{}", EMPTY_FOLDER_HASH);
        assert_eq!(
            ipfs_root_str,
            Root::from_str(&ipfs_root_str).unwrap().to_string()
        );
    }

    #[test]
    fn root_dnslink_ok() {
        let dnslink_root_str = "/ipns/bootstrap.libp2p.io";
        assert_eq!(
            dnslink_root_str,
            Root::from_str(dnslink_root_str).unwrap().to_string()
        );
    }

    #[test]
    fn root_dnslink_with_invalid_domain_err() {
        let dnslink_root_str = "/ipns/notadomain.123$$$%@";
        assert_eq!(
            PathParseError::DnsLinkDomainInvalid("notadomain.123$$$%@".to_string()),
            Root::from_str(dnslink_root_str).unwrap_err()
        );
    }

    #[test]
    fn root_dnslink_with_non_uts46_conformant_err() {
        let dnslink_root_str = "/ipns/Ⅎ.com";
        assert_eq!(
            PathParseError::DnsLinkDomainInvalid("Ⅎ.com".to_string()),
            Root::from_str(dnslink_root_str).unwrap_err()
        );
    }

    #[test]
    fn root_ipns_ok() {
        let ipns_root_str = "/ipns/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN";
        assert_eq!(
            ipns_root_str,
            Root::from_str(ipns_root_str).unwrap().to_string()
        );
    }

    #[test]
    fn ipfs_path_without_suffix_ok() {
        let path_string = format!("/ipfs/{}", EMPTY_FOLDER_HASH);
        assert_eq!(
            path_string,
            Path::from_str(&path_string).unwrap().to_string()
        );
    }

    #[test]
    fn ipfs_path_with_suffix_ok() {
        let path_string = "/ipfs/QmXGuztteR8h7TKDsw61yCrwYzrw8kcfQMfG8dXd3Y2ZkC/spec/ipfs.rs";
        assert_eq!(
            path_string,
            Path::from_str(&path_string).unwrap().to_string()
        );
    }

    #[test]
    fn ipfs_path_with_dot_dot_to_no_suffix_ok() {
        let path_string =
            "/ipfs/QmXGuztteR8h7TKDsw61yCrwYzrw8kcfQMfG8dXd3Y2ZkC/../spec/ipfs.rs/../../../../../";
        assert_eq!(
            "/ipfs/QmXGuztteR8h7TKDsw61yCrwYzrw8kcfQMfG8dXd3Y2ZkC",
            Path::from_str(&path_string).unwrap().to_string(),
        );
    }

    #[test]
    fn ipfs_path_with_invalid_cid_err() {
        let path_string = "/ipfs/QmSomeHash";
        assert_eq!(
            PathParseError::CidError(cid::Error::ParsingError),
            Path::from_str(&path_string).unwrap_err(),
        );
    }

    #[test]
    fn ipfs_path_with_dot_dot_to_some_suffix_ok() {
        let path_string = "/ipfs/QmXGuztteR8h7TKDsw61yCrwYzrw8kcfQMfG8dXd3Y2ZkC/spec/ipfs.rs/../";
        assert_eq!(
            "/ipfs/QmXGuztteR8h7TKDsw61yCrwYzrw8kcfQMfG8dXd3Y2ZkC/spec",
            Path::from_str(&path_string).unwrap().to_string(),
        );
    }

    #[test]
    fn ipfs_result_err_ser_der_ok() {
        let expect = Result::<bool>::Err(Error {
            message: "invalid 'ipfs ref' path".to_string(),
            code: 0,
            Type: "error".to_string(),
        });

        assert_eq!(
            include_str!("./test/ipfs_result_err.json"),
            serde_json::to_string(&expect).unwrap(),
        );

        assert_eq!(
            serde_json::from_str::<'static, Result<bool>>(include_str!(
                "./test/ipfs_result_err.json"
            ))
            .unwrap(),
            expect,
        );
    }

    #[test]
    fn ipfs_result_ok_add_response_der_ok() {
        use cid::ToCid;
        let expect = Result::<AddResponse>::Ok(AddResponse {
            name: "empty".to_string(),
            hash: Cid("QmbFMke1KXqnYyBBWxB74N4c5SBnJMVAiMNRcGu6x1AwQH"
                .to_cid()
                .unwrap()),
            size: "6".to_string(),
        });

        assert_eq!(
            serde_json::from_str::<'static, Result<AddResponse>>(include_str!(
                "./test/ipfs_result_ok_add_response.json"
            ))
            .unwrap(),
            expect,
        );
    }

    #[test]
    fn ipfs_key_list_response_der_ok() {
        use cid::ToCid;
        let expect = KeyListResponse {
            keys: vec![Key {
                name: "self".to_string(),
                id: Cid("QmcEiVtvhFAKDqA7ZVSDYwR6AMVoKNwuhTwcX7rdqF5AN5"
                    .to_cid()
                    .unwrap()),
            }],
        };

        assert_eq!(
            serde_json::from_str::<'static, KeyListResponse>(include_str!(
                "./test/ipfs_key_list_response.json"
            ))
            .unwrap(),
            expect,
        );
    }
}
