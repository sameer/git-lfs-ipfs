use regex::Regex;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use lazy_static::lazy_static;

use std::fmt;

const VERSION: &str = "https://git-lfs.github.com/spec/v1";
const KEY_REGEX_STR: &str = "[a-z0-9.-]+";

lazy_static! {
    static ref KEY_REGEX: Regex = Regex::new(KEY_REGEX_STR).unwrap();
}

#[derive(Debug)]
pub struct Pointer {
    lines: Vec<Line>,
}

impl Default for Pointer {
    fn default() -> Self {
        Self {
            lines: vec![Line {
                key: "version".to_string(),
                value: VERSION.to_string(),
            }],
        }
    }
}

impl Serialize for Pointer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(
            self.lines
                .iter()
                .fold(String::new(), |mut acc, line| {
                    acc.push_str(format!("{} {}\n", line.key, line.value).as_str());
                    acc
                })
                .as_str(),
        )
    }
}

impl<'de> Deserialize<'de> for Pointer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PointerVisitor;
        impl<'de> de::Visitor<'de> for PointerVisitor {
            type Value = Pointer;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a string in the format <key> <value>")
            }

            fn visit_str<E>(self, lines_str: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let lines: Vec<Line> =
                    lines_str
                        .split_terminator('\n')
                        .try_fold(vec![], |mut acc, line_str| {
                            let mut it = line_str.split_whitespace();
                            let key = it.next().ok_or_else(|| de::Error::missing_field("key"))?;
                            if !KEY_REGEX.is_match(key) {
                                return Err(de::Error::invalid_value(
                                    de::Unexpected::Str(key),
                                    &format!("expected key to match regex {}", KEY_REGEX.as_str())
                                        .as_str(),
                                ));
                            }
                            let value =
                                it.next().ok_or_else(|| de::Error::missing_field("value"))?;
                            let count = it.count();
                            if count != 2 {
                                return Err(de::Error::invalid_length(
                                    count,
                                    &"expected 2 elements",
                                ));
                            }
                            acc.push(Line {
                                key: key.to_string(),
                                value: value.to_string(),
                            });
                            Ok(acc)
                        })?;
                Ok(Pointer { lines })
            }
        }
        deserializer.deserialize_string(PointerVisitor)
    }
}

#[derive(Debug)]
pub struct Line {
    key: String,
    value: String,
}

impl Serialize for Line {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(format!("{} {}", self.key, self.value).as_str())
    }
}

impl<'de> Deserialize<'de> for Line {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LineVisitor;
        impl<'de> de::Visitor<'de> for LineVisitor {
            type Value = Line;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a string in the format <key> <value>")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let mut it = v.split_whitespace();
                let key = it.next().ok_or_else(|| de::Error::missing_field("key"))?;
                if !KEY_REGEX.is_match(key) {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Str(key),
                        &format!("expected key to match regex {}", KEY_REGEX.as_str()).as_str(),
                    ));
                }
                let value = it.next().ok_or_else(|| de::Error::missing_field("value"))?;
                let count = it.count();
                if count != 2 {
                    return Err(de::Error::invalid_length(count, &"expected 2 elements"));
                }
                Ok(Line {
                    key: key.to_string(),
                    value: value.to_string(),
                })
            }
        }
        deserializer.deserialize_string(LineVisitor)
    }
}
