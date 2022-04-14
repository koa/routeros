use mac_address::MacAddress;
use mac_address::MacParseError;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::net::{AddrParseError, IpAddr, Ipv4Addr};
use std::num::ParseIntError;
use std::ops::{Deref, DerefMut, RangeInclusive};
use std::str::{FromStr, ParseBoolError};

use crate::client::api::RosError;

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
    pub fn set(&mut self, value: T) {
        self.current_value = Some(value);
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

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Duration {
    milliseconds: u32,
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

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct IpNetAddr {
    ip: IpAddr,
    netmask: u8,
}

impl IpNetAddr {
    pub fn new(ip: IpAddr, netmask: u8) -> IpNetAddr {
        IpNetAddr { ip, netmask }
    }
    pub fn ip(&self) -> &IpAddr {
        &self.ip
    }
    pub fn netmask(&self) -> u8 {
        self.netmask
    }
}

impl Default for IpNetAddr {
    fn default() -> Self {
        IpNetAddr {
            ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            netmask: 0,
        }
    }
}

impl FromStr for IpNetAddr {
    type Err = RosError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_api(s)
    }
}

impl RosValue for IpNetAddr {
    type Type = IpNetAddr;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        let mut split = value.split('/');
        if let Some(ip_addr_string) = split.next() {
            let ip = IpAddr::from_api(ip_addr_string)?;
            let netmask: u8 = if let Some(netmask) = split.next() {
                netmask.parse()?
            } else {
                match ip {
                    IpAddr::V4(_) => 32,
                    IpAddr::V6(_) => 128,
                }
            };
            if split.next().is_some() {
                Result::Err(RosError::SimpleMessage(format!(
                    "Network address has more than 1 '/': {value}"
                )))
            } else {
                Result::Ok(IpNetAddr { ip, netmask })
            }
        } else {
            Result::Err(RosError::SimpleMessage(format!(
                "Cannot split network address: {value}"
            )))
        }
    }

    fn to_api(&self, _: &ValueFormat) -> String {
        format!(
            "{address}/{netmask}",
            address = self.ip,
            netmask = self.netmask
        )
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
            milliseconds: milli_second_count as u32
                + second_count as u32 * 1000
                + minute_count as u32 * 1000 * 60
                + hour_count as u32 * 1000 * 3600,
        })
    }

    fn to_api(&self, _: &ValueFormat) -> String {
        if self.milliseconds < 1000 && self.milliseconds != 0 {
            format!("{}ms", self.milliseconds)
        } else {
            let all_seconds = self.milliseconds / 1000;
            let seconds = all_seconds % 60;
            let minutes = (all_seconds / 60) % 60;
            let hours = all_seconds / 3600;
            format!("{seconds:02}:{minutes:02}:{hours:02}")
        }
    }
}

impl Default for Duration {
    fn default() -> Self {
        Duration { milliseconds: 0 }
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
