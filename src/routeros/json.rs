use serde::de::{Error, Visitor};
use serde::{Deserializer, Serializer};
use std::collections::HashSet;
use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::RangeInclusive;
use std::str::FromStr;

struct StrBoolVisitor {}

impl<'a> Visitor<'a> for StrBoolVisitor {
    type Value = &'a str;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a borrowed string")
    }

    fn visit_borrowed_str<E>(self, v: &'a str) -> std::result::Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v) // so easy
    }

    /*
    fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> std::result::Result<Self::Value, E>
    where
        E: Error,
    {
        str::from_utf8(v).map_err(|_| Error::invalid_value(Unexpected::Bytes(v), &self))
    }

     */
}

fn stringvec_from_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let ret = deserializer.deserialize_str(StrBoolVisitor {})?;
    if ret.len() == 0 {
        Ok(vec![])
    } else {
        Ok(ret.split(",").map(|v| v.to_string()).collect())
    }
}

pub fn stringset_from_string<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let ret = deserializer.deserialize_str(StrBoolVisitor {})?;
    if ret.len() == 0 {
        Ok(HashSet::new())
    } else {
        Ok(ret.split(",").map(|v| v.to_string()).collect())
    }
}

pub fn serialize_stringset_to_string<'de, S>(
    value: &HashSet<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if value.is_empty() {
        serializer.serialize_str("")
    } else {
        let mut result = String::new();
        for part in value {
            if result.len() > 0 {
                result.push(',');
            }
            result.push_str(part);
        }
        serializer.serialize_str(result.as_str())
    }
}

pub fn deserialize_number_ranges_from_string<'de, D, N, E>(
    deserializer: D,
) -> Result<Vec<RangeInclusive<N>>, D::Error>
where
    D: Deserializer<'de>,
    N: FromStr<Err = E> + Copy,
    E: Debug + Display,
{
    let ret = deserializer.deserialize_str(StrBoolVisitor {})?;
    let vec: Result<Vec<RangeInclusive<N>>, E> = ret
        .split(",")
        .map(|v| {
            let split_pos = v.find("-");
            match split_pos {
                Some(pos) => {
                    let n1 = &v[..pos];
                    let n2 = &v[(pos + 1)..];
                    let start = n1.parse()?;
                    let end = n2.parse()?;
                    Ok(RangeInclusive::new(start, end))
                }
                None => {
                    let v = v.parse()?;
                    Ok(RangeInclusive::new(v, v))
                }
            }
        })
        .collect();
    return vec.map_err(Error::custom);
}

pub fn serialize_number_ranges_to_string<'de, S, N>(
    value: &Vec<RangeInclusive<N>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    N: ToString + Copy + Eq,
{
    if value.is_empty() {
        serializer.serialize_str("")
    } else {
        let result = value
            .iter()
            .map(|r| {
                let start = r.start();
                let end = r.end();
                if start == end {
                    start.to_string()
                } else {
                    format!("{}-{}", start.to_string(), end.to_string())
                }
            })
            .collect::<Vec<String>>()
            .join(",");
        serializer.serialize_str(result.as_str())
    }
}

pub fn deserialize_optional_from_string<'de, D, N, E>(
    deserializer: D,
) -> Result<Option<N>, D::Error>
where
    D: Deserializer<'de>,
    N: FromStr<Err = E> + Copy,
    E: Debug + Display,
{
    deserializer
        .deserialize_str(StrBoolVisitor {})?
        .parse()
        .map_err(Error::custom)
        .map(Some)
}

pub fn serialize_optional_to_string<'de, S, N>(
    value: &Option<N>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    N: ToString + Copy,
{
    if let Some(value) = value {
        serializer.serialize_str(value.to_string().as_str())
    } else {
        serializer.serialize_none()
    }
}
