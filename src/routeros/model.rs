use std::collections::hash_set::Iter;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::iter::Map;
use std::net::IpAddr;
use std::ops::{Deref, RangeInclusive};

use crate::routeros::client::api::RosError;

#[derive(Debug, Default, Clone)]
pub struct RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    original_value: String,
    current_value: Option<T>,
}

pub trait RosValue: Eq {
    type Type;
    type Err: Into<RosError>;
    fn from_api(value: &str) -> Result<Self::Type, Self::Err>;
    fn to_api(&self) -> String;
}

pub trait RosFieldAccessor {
    fn modified_value(&self) -> Option<String>;
    fn set_from_api(&mut self, value: &str) -> Result<(), RosError>;
    fn clear(&mut self) -> Result<(), RosError>;
    fn reset(&mut self) -> Result<(), RosError>;
}
impl<T> Display for RosFieldValue<T>
where
    T: RosValue<Type = T>,
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(value) = self.current_value.as_ref() {
            value.fmt(f)
        } else {
            f.write_str("<empty>")
            //Ok(())
        }
    }
}

impl<T> RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    pub fn get(&self) -> &Option<T> {
        &self.current_value
    }
    pub fn set(&mut self, value: Option<T>) {
        self.current_value = value;
    }
    pub fn original_value(&self) -> String {
        self.original_value.clone()
    }
    fn original_value_converted(&self) -> Option<T> {
        if self.original_value.is_empty() {
            Option::None
        } else {
            match T::from_api(&self.original_value) {
                Ok(value) => Option::Some(value),
                Err(_) => Option::None,
            }
        }
    }
}

impl<T> RosFieldAccessor for RosFieldValue<T>
where
    T: RosValue<Type = T> + Eq,
    // Err: Into<RosError>,
{
    fn modified_value(&self) -> Option<String> {
        let original_value = self.original_value_converted();
        if original_value == self.current_value {
            return Option::None;
        }
        let new_value = self
            .current_value
            .as_ref()
            .map(|v| v.to_api())
            .unwrap_or_else(|| String::from(""));
        if new_value.ne(self.original_value.as_str()) {
            Some(new_value)
        } else {
            None
        }
    }

    fn set_from_api(&mut self, value: &str) -> Result<(), RosError> {
        self.original_value = value.to_string();
        self.reset()
    }

    fn clear(&mut self) -> Result<(), RosError> {
        self.current_value = None;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), RosError> {
        if self.original_value.is_empty() {
            self.current_value = None;
        } else {
            self.current_value =
                Some(T::from_api(self.original_value.as_str()).map_err(|e| e.into())?);
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Auto<V>
where
    V: RosValue,
{
    Auto,
    Value(V),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Duration {
    seconds: u32,
}

impl<T> Deref for RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    type Target = Option<T::Type>;
    fn deref(&self) -> &Self::Target {
        &self.current_value
    }
}

impl<RV> RosValue for HashSet<RV>
where
    RV: RosValue<Type = RV> + Eq + Hash,
{
    type Type = HashSet<RV>;
    type Err = RV::Err;

    /*
    fn empty() -> Self::Type {
        HashSet::new()
    }

     */

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        let mut ret: HashSet<RV> = HashSet::new();

        for part in value.split(",") {
            let entry = RV::from_api(part)?;
            ret.insert(entry);
        }
        Ok(ret)
    }

    fn to_api(&self) -> String {
        let map: Map<Iter<RV>, fn(&RV) -> String> = self.iter().map(RV::to_api);
        let mut ret: Option<String> = None;
        for part in map {
            if let Some(buffer) = ret.as_mut() {
                buffer.push(',');
                buffer.push_str(&part);
            } else {
                ret = Some(part);
            }
        }
        ret.unwrap_or_default()
    }
}

impl RosValue for bool {
    type Type = bool;
    type Err = RosError;
    fn from_api(value: &str) -> Result<bool, Self::Err> {
        value.parse().map_err(RosError::from)
    }

    fn to_api(&self) -> String {
        self.to_string()
    }
}

impl RosValue for String {
    type Type = String;
    type Err = RosError;
    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        Ok(String::from(value))
    }

    fn to_api(&self) -> String {
        self.clone()
    }
}

impl<V> RosValue for RangeInclusive<V>
where
    V: RosValue<Type = V> + Copy + Eq,
{
    type Type = RangeInclusive<V>;
    type Err = V::Err;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        Ok(if let Some(split) = value.find('-') {
            let start = V::from_api(&value[0..split])?;
            let end = V::from_api(&value[(split + 1)..])?;
            RangeInclusive::new(start, end)
        } else {
            let value = V::from_api(value)?;
            RangeInclusive::new(value, value)
        })
    }

    fn to_api(&self) -> String {
        let start = self.start();
        let end = self.end();
        if start == end {
            start.to_api()
        } else {
            format!("{}-{}", start.to_api(), end.to_api())
        }
    }
}

impl RosValue for u16 {
    type Type = u16;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        if value.starts_with("0x") {
            u16::from_str_radix(&value[2..], 16)
        } else {
            value.parse()
        }
        .map_err(RosError::from)
    }

    fn to_api(&self) -> String {
        self.to_string()
    }
}
impl RosValue for u8 {
    type Type = u8;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        if value.starts_with("0x") {
            u8::from_str_radix(&value[2..], 16)
        } else {
            value.parse()
        }
        .map_err(RosError::from)
    }

    fn to_api(&self) -> String {
        self.to_string()
    }
}
impl RosValue for u32 {
    type Type = u32;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        if value.starts_with("0x") {
            u32::from_str_radix(&value[2..], 16)
        } else {
            value.parse()
        }
        .map_err(RosError::from)
    }

    fn to_api(&self) -> String {
        self.to_string()
    }
}
impl RosValue for u64 {
    type Type = u64;
    type Err = RosError;
    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        value.parse().map_err(RosError::from)
    }

    fn to_api(&self) -> String {
        self.to_string()
    }
}
impl RosValue for Option<u32> {
    type Type = Option<u32>;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        if value == "none" {
            Ok(Option::None)
        } else if value.starts_with("0x") {
            u32::from_str_radix(&value[2..], 16)
                .map(Option::Some)
                .map_err(RosError::from)
        } else {
            value.parse().map(Option::Some).map_err(RosError::from)
        }
    }

    fn to_api(&self) -> String {
        if let Some(value) = self {
            value.to_string()
        } else {
            String::from("none")
        }
    }
}

impl<V> RosValue for Auto<V>
where
    V: RosValue<Type = V>,
    RosError: From<<V as RosValue>::Err>,
{
    type Type = Auto<V>;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        if value == "auto" {
            Ok(Auto::Auto)
        } else {
            Ok(Auto::Value(V::from_api(value)?))
        }
    }

    fn to_api(&self) -> String {
        match self {
            Auto::Auto => String::from("auto"),
            Auto::Value(value) => value.to_api(),
        }
    }
}

impl<V: RosValue> Default for Auto<V> {
    fn default() -> Self {
        Auto::Auto
    }
}

impl RosValue for Duration {
    type Type = Duration;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        let mut second_count: u8 = 0;
        let mut minute_count: u8 = 0;
        let mut hour_count: u8 = 0;
        let mut positional_count: Vec<u8> = Vec::new();
        let mut number = String::new();
        for ch in value.chars() {
            if ch.is_digit(10) {
                number.push(ch);
            } else if ch == ':' {
                positional_count.push(number.parse()?);
                number.clear();
            } else if ch == 'h' {
                hour_count = number.parse()?;
                number.clear();
            } else if ch == 'm' {
                minute_count = number.parse()?;
                number.clear();
            } else if ch == 's' {
                second_count = number.parse()?;
                number.clear();
            };
        }
        positional_count.reverse();
        if let Some(count) = positional_count.get(0) {
            second_count += count;
        }
        if let Some(count) = positional_count.get(1) {
            minute_count += count;
        }
        if let Some(count) = positional_count.get(2) {
            hour_count += count;
        }
        Ok(Duration {
            seconds: second_count as u32 + minute_count as u32 * 60 + hour_count as u32 * 3600,
        })
    }

    fn to_api(&self) -> String {
        let seconds = self.seconds % 60;
        let minutes = (self.seconds / 60) % 60;
        let hours = self.seconds / 3600;
        format!("{seconds:02}:{minutes:02}:{hours:02}")
    }
}

impl Default for Duration {
    fn default() -> Self {
        Duration { seconds: 0 }
    }
}

pub trait ResourceBuilder<R>: Send + Sync
where
    R: RouterOsResource + Sized,
{
    fn write_field<K, V>(&mut self, key: K, value: V) -> Result<(), RosError>
    where
        K: AsRef<str>,
        V: AsRef<str>,
        V: ToString;
    fn build(self) -> R;
}

pub trait RouterOsApiFieldAccess {
    fn fields_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut dyn RosFieldAccessor)> + '_>;
    fn fields(&self) -> Box<dyn Iterator<Item = (&str, &dyn RosFieldAccessor)> + '_>;
}

pub trait RouterOsResource:
    Sized + Debug + Send + Default + RouterOsApiFieldAccess + Clone
{
    fn resource_path() -> &'static str;
    fn id_field() -> Option<&'static str> {
        Option::None
    }
    fn id_value(&self) -> Option<String> {
        Option::None
    }
    fn resource_url(ip_addr: IpAddr) -> String {
        format!("https://{}/rest/{}", ip_addr, Self::resource_path())
    }
    fn is_modified(&self) -> bool {
        return self.fields().any(|e| e.1.modified_value().is_some());
    }
}

//pub trait RouterOsSingleResource: RouterOsResource + DeserializeOwned {}
//pub trait RouterOsListResource: RouterOsResource + DeserializeOwned {}

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
