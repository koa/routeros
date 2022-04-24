use std::collections::HashSet;
use std::fmt::{format, Debug, Display, Formatter, Write};
use std::hash::Hash;
use std::net::{AddrParseError, IpAddr};
use std::num::ParseIntError;
use std::ops::RangeInclusive;
use std::str::{FromStr, ParseBoolError};
use std::time::Duration;

use ipnet::IpNet;
use mac_address::MacAddress;
use mac_address::MacParseError;

use crate::RosError;

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

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Auto<V>
where
    V: RosValue,
{
    Auto,
    Value(V),
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

impl RosValue for IpNet {
    type Type = IpNet;
    type Err = ipnet::AddrParseError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        value.parse()
    }

    fn to_api(&self, _format: &ValueFormat) -> String {
        self.to_string()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct IpWithInterface {
    ip: IpAddr,
    interface: String,
}

impl IpWithInterface {
    pub fn get_ip(&self) -> IpAddr {
        self.ip
    }
    pub fn get_interface(&self) -> &str {
        self.interface.as_str()
    }
}

impl Display for IpWithInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.ip, f)?;
        f.write_char('%')?;
        std::fmt::Display::fmt(&self.interface, f)?;
        Ok(())
    }
}

impl RosValue for IpWithInterface {
    type Type = IpWithInterface;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        value.parse()
    }

    fn to_api(&self, _format: &ValueFormat) -> String {
        self.to_string()
    }
}

impl FromStr for IpWithInterface {
    type Err = RosError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(split_pos) = s.find('%') {
            Ok(IpWithInterface {
                ip: s[0..split_pos].parse()?,
                interface: s[split_pos + 1..].to_owned(),
            })
        } else {
            Err(RosError::SimpleMessage(format!("Cannot parse {s}")))
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum IpOrInterface {
    Ip(IpAddr),
    Interface(String),
    IpWithInterface(IpWithInterface),
}

impl IpOrInterface {
    pub fn ip(&self) -> Option<IpAddr> {
        match self {
            IpOrInterface::Ip(ip) => Some(*ip),
            IpOrInterface::Interface(_) => None,
            IpOrInterface::IpWithInterface(if_data) => Some(if_data.get_ip()),
        }
    }
    pub fn if_name(&self) -> Option<&str> {
        match self {
            IpOrInterface::Ip(_) => None,
            IpOrInterface::Interface(if_name) => Some(if_name),
            IpOrInterface::IpWithInterface(if_data) => Some(if_data.get_interface()),
        }
    }
}

impl From<IpAddr> for IpOrInterface {
    fn from(ip: IpAddr) -> Self {
        IpOrInterface::Ip(ip)
    }
}

impl From<IpWithInterface> for IpOrInterface {
    fn from(ip: IpWithInterface) -> Self {
        IpOrInterface::IpWithInterface(ip)
    }
}

impl From<String> for IpOrInterface {
    fn from(if_name: String) -> Self {
        IpOrInterface::Interface(if_name)
    }
}

impl RosValue for IpOrInterface {
    type Type = IpOrInterface;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        value.parse()
    }

    fn to_api(&self, _format: &ValueFormat) -> String {
        self.to_string()
    }
}

impl Display for IpOrInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IpOrInterface::Ip(ip) => Display::fmt(ip, f),
            IpOrInterface::Interface(if_name) => Display::fmt(if_name, f),
            IpOrInterface::IpWithInterface(v) => Display::fmt(v, f),
        }
    }
}

impl FromStr for IpOrInterface {
    type Err = RosError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = IpWithInterface::from_str(s) {
            Ok(IpOrInterface::IpWithInterface(value))
        } else if let Ok(ip) = IpAddr::from_str(s) {
            Ok(IpOrInterface::Ip(ip))
        } else {
            Ok(IpOrInterface::Interface(String::from(s)))
        }
    }
}
