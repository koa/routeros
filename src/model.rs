use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::net::{AddrParseError, IpAddr, Ipv4Addr};
use std::num::ParseIntError;
use std::ops::{Deref, DerefMut, RangeInclusive};
use std::str::{FromStr, ParseBoolError};
use std::time::Duration;

use mac_address::MacAddress;
use mac_address::MacParseError;

use crate::RosError;
pub mod inet;

#[derive(Debug, Clone)]
pub struct RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    original_value: String,
    current_value: Option<T>,
}

impl<T> Default for RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    fn default() -> Self {
        Self {
            original_value: String::new(),
            current_value: Option::None,
        }
    }
}

pub enum ValueFormat {
    Api,
    Cli,
}

pub trait RosValue: Eq {
    type Type;
    type Err: Into<RosError>;
    fn from_api(value: &str) -> Result<Self::Type, Self::Err>;
    fn to_api(&self, format: &ValueFormat) -> String;
}

pub trait RosFieldAccessor {
    fn modified_value(&self, format: &ValueFormat) -> Option<String>;
    fn api_value(&self, format: &ValueFormat) -> String;
    fn set_from_api(&mut self, value: &str) -> Result<(), RosError>;
    fn clear(&mut self) -> Result<(), RosError>;
    fn reset(&mut self) -> Result<(), RosError>;
    fn has_value(&self) -> bool;
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
    pub fn set<IT>(&mut self, value: IT)
    where
        IT: Into<T>,
    {
        self.current_value = Some(value.into());
    }
    pub fn clear(&mut self) {
        self.current_value = None;
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
    fn modified_value(&self, format: &ValueFormat) -> Option<String> {
        let original_value = self.original_value_converted();
        if original_value == self.current_value {
            return Option::None;
        }
        let new_value = self.api_value(format);
        if new_value.ne(self.original_value.as_str()) {
            Some(new_value)
        } else {
            None
        }
    }

    fn api_value(&self, format: &ValueFormat) -> String {
        self.current_value
            .as_ref()
            .map(|v| v.to_api(format))
            .unwrap_or_else(|| String::from(""))
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

    fn has_value(&self) -> bool {
        self.current_value.is_some()
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

impl<T> Deref for RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    type Target = Option<T::Type>;
    fn deref(&self) -> &Self::Target {
        &self.current_value
    }
}

impl<T> DerefMut for RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current_value
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

    fn to_api(&self, value_format: &ValueFormat) -> String {
        let mut ret: Option<String> = None;
        for part_ref in self.iter() {
            let part = part_ref.to_api(value_format);
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
    type Err = ParseBoolError;
    fn from_api(value: &str) -> Result<bool, Self::Err> {
        match value {
            "no" => Ok(false),
            "yes" => Ok(true),
            value => value.parse(),
        }
    }

    fn to_api(&self, format: &ValueFormat) -> String {
        match *format {
            ValueFormat::Api => self.to_string(),
            ValueFormat::Cli => String::from(match *self {
                true => "yes",
                false => "no",
            }),
        }
    }
}

impl RosValue for String {
    type Type = String;
    type Err = RosError;
    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        Ok(String::from(value))
    }

    fn to_api(&self, _: &ValueFormat) -> String {
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

    fn to_api(&self, format: &ValueFormat) -> String {
        let start = self.start();
        let end = self.end();
        if start == end {
            start.to_api(format)
        } else {
            format!("{}-{}", start.to_api(format), end.to_api(format))
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

    fn to_api(&self, _: &ValueFormat) -> String {
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

    fn to_api(&self, _: &ValueFormat) -> String {
        self.to_string()
    }
}

impl RosValue for i8 {
    type Type = i8;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        if value.starts_with("0x") {
            i8::from_str_radix(&value[2..], 16)
        } else {
            value.parse()
        }
        .map_err(RosError::from)
    }

    fn to_api(&self, _: &ValueFormat) -> String {
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

    fn to_api(&self, _: &ValueFormat) -> String {
        self.to_string()
    }
}

impl RosValue for u64 {
    type Type = u64;
    type Err = RosError;
    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        value.parse().map_err(RosError::from)
    }

    fn to_api(&self, _: &ValueFormat) -> String {
        self.to_string()
    }
}

impl RosValue for Option<u32> {
    type Type = Option<u32>;
    type Err = ParseIntError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        if value == "none" {
            Ok(Option::None)
        } else if value.starts_with("0x") {
            u32::from_str_radix(&value[2..], 16).map(Option::Some)
        } else {
            value.parse().map(Option::Some)
        }
    }

    fn to_api(&self, _: &ValueFormat) -> String {
        if let Some(value) = self {
            value.to_string()
        } else {
            String::from("none")
        }
    }
}

impl RosValue for IpAddr {
    type Type = IpAddr;
    type Err = AddrParseError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        value.parse()
    }

    fn to_api(&self, _: &ValueFormat) -> String {
        self.to_string()
    }
}

impl RosValue for MacAddress {
    type Type = MacAddress;
    type Err = MacParseError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        value.parse()
    }

    fn to_api(&self, _: &ValueFormat) -> String {
        self.to_string()
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

    fn to_api(&self, format: &ValueFormat) -> String {
        match self {
            Auto::Auto => String::from("auto"),
            Auto::Value(value) => value.to_api(format),
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
        let mut milli_second_count: u16 = 0;
        let mut second_count: u8 = 0;
        let mut minute_count: u8 = 0;
        let mut hour_count: u8 = 0;
        let mut day_count: u16 = 0;
        let mut positional_count: Vec<u8> = Vec::new();
        let mut number = String::new();
        let mut last_was_m = false;
        for ch in value.chars() {
            if ch.is_digit(10) {
                if last_was_m {
                    minute_count = number.parse()?;
                    number.clear();
                }
                number.push(ch);
                last_was_m = false;
            } else if ch == ':' {
                positional_count.push(number.parse()?);
                last_was_m = false;
                number.clear();
            } else if ch == 'h' {
                hour_count = number.parse()?;
                last_was_m = false;
                number.clear();
            } else if ch == 'm' {
                last_was_m = true;
            } else if ch == 's' {
                if last_was_m {
                    milli_second_count = number.parse()?;
                } else {
                    second_count = number.parse()?;
                }
                last_was_m = false;
                number.clear();
            } else if ch == 'd' {
                day_count = number.parse()?;
                last_was_m = false;
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

        Ok(Duration::from_millis(
            (second_count as u64
                + minute_count as u64 * 60
                + hour_count as u64 * 3600
                + day_count as u64 * 3600 * 24)
                * 1000
                + milli_second_count as u64,
        ))
    }

    fn to_api(&self, _: &ValueFormat) -> String {
        let remaining_millis = self.subsec_millis();
        let all_seconds = self.as_secs();
        let seconds = all_seconds % 60;
        let minutes = (all_seconds / 60) % 60;
        let hours = all_seconds / 3600;
        let days = hours / 24;
        let remaining_hours = days % 24;
        let mut ret = String::new();
        if days > 0 {
            ret.push_str(&format!("{days}d"));
        }
        if remaining_hours > 0 {
            ret.push_str(&format!("{remaining_hours}h"));
        }
        if minutes > 0 {
            ret.push_str(&format!("{minutes}m"));
        }
        if seconds > 0 {
            ret.push_str(&format!("{seconds}s"));
        }
        if remaining_millis > 0 {
            ret.push_str(&format!("{remaining_millis}ms"));
        }
        if ret.is_empty() {
            String::from("0s")
        } else {
            ret
        }
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

#[derive(PartialEq)]
pub struct FieldDescription {
    pub name: &'static str,
    pub is_read_only: bool,
    pub is_id: bool,
}

pub trait RouterOsApiFieldAccess {
    fn fields_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = (&'static FieldDescription, &mut dyn RosFieldAccessor)> + '_>;
    fn fields(
        &self,
    ) -> Box<dyn Iterator<Item = (&'static FieldDescription, &dyn RosFieldAccessor)> + '_>;
    fn is_dynamic(&self) -> bool {
        false
    }
}

pub trait RouterOsResource:
    Sized + Debug + Send + Default + RouterOsApiFieldAccess + Clone
{
    fn resource_path() -> &'static str;

    fn resource_url(ip_addr: IpAddr) -> String {
        format!("https://{}/rest/{}", ip_addr, Self::resource_path())
    }
    fn is_modified(&self) -> bool {
        return self
            .fields()
            .any(|e| e.1.modified_value(&ValueFormat::Api).is_some());
    }
    fn id_field(&self) -> Option<(&'static FieldDescription, &dyn RosFieldAccessor)> {
        self.fields()
            .filter(|(description, value)| description.is_id && value.has_value())
            .next()
    }
}

pub trait RouterOsListResource: RouterOsResource {}

pub trait RouterOsSingleResource: RouterOsResource {}
