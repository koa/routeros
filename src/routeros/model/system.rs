use serde::{Deserialize, Serialize};
use std::alloc::System;

use crate::routeros::json::{
    deserialize_number_ranges_from_string, deserialize_optional_from_string,
    serialize_number_ranges_to_string, serialize_optional_to_string, serialize_stringset_to_string,
    stringset_from_string,
};
use crate::routeros::model::{ResourceBuilder, RouterOsResource};
use std::fmt::Debug;
use std::thread::Builder;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct SystemResource {
    architecture_name: Option<String>,
    board_name: Option<String>,
    cpu: Option<String>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    cpu_frequency: Option<u64>,
    factory_software: Option<String>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    free_memory: Option<u64>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    total_hdd_space: Option<u64>,
    uptime: Option<String>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    write_sect_since_reboot: Option<u64>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    bad_blocks: Option<u64>,
    build_time: Option<String>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    cpu_count: Option<u16>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    cpu_load: Option<u8>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    free_hdd_space: Option<u64>,
    platform: Option<String>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    total_memory: Option<u64>,
    version: Option<String>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    write_sect_total: Option<u64>,
}

//impl RouterOsSingleResource for SystemResource {}

struct SystemBuilder {
    data: SystemResource,
}

impl ResourceBuilder<SystemResource> for SystemResource {
    fn write_field<K, V>(&mut self, key: K, value: V) -> bool
    where
        K: AsRef<str>,
        V: AsRef<str>,
        V: ToString,
    {
        match key.as_ref() {
            "architecture-name" => self.architecture_name = Some(value.to_string()),
            "board-name" => self.board_name = Some(value.to_string()),
            "cpu" => self.cpu = Some(value.to_string()),
            "cpu-frequency" => self.cpu_frequency = Some(value.as_ref().parse().unwrap()),
            "factory-software" => self.factory_software = Some(value.to_string()),
            "free-memory" => self.free_memory = Some(value.as_ref().parse().unwrap()),
            "total-hdd-space" => self.total_hdd_space = Some(value.as_ref().parse().unwrap()),
            "uptime" => self.uptime = Some(value.to_string()),
            "write-sect-since-reboot" => {
                self.write_sect_since_reboot = Some(value.as_ref().parse().unwrap())
            }
            "bad-blocks" => self.bad_blocks = Some(value.as_ref().parse().unwrap()),
            "build-time" => self.build_time = Some(value.to_string()),
            "cpu-count" => self.cpu_count = Some(value.as_ref().parse().unwrap()),
            "cpu-load" => self.cpu_load = Some(value.as_ref().parse().unwrap()),
            "free-hdd-space" => self.free_hdd_space = Some(value.as_ref().parse().unwrap()),
            "platform" => self.platform = Some(value.as_ref().parse().unwrap()),
            "total-memory" => self.total_memory = Some(value.as_ref().parse().unwrap()),
            "version" => self.version = Some(value.as_ref().parse().unwrap()),
            "write-sect-total" => self.write_sect_total = Some(value.as_ref().parse().unwrap()),
            key => {
                println!("Unknown key: {}", key);
                return false;
            }
        }
        true
    }

    fn build(self) -> SystemResource {
        self
    }
}

impl RouterOsResource for SystemResource {
    type Builder = SystemResource;

    fn resource_path() -> &'static str {
        "system/resource"
    }

    fn builder() -> Self::Builder {
        SystemResource {
            architecture_name: None,
            board_name: None,
            cpu: None,
            cpu_frequency: None,
            factory_software: None,
            free_memory: None,
            total_hdd_space: None,
            uptime: None,
            write_sect_since_reboot: None,
            bad_blocks: None,
            build_time: None,
            cpu_count: None,
            cpu_load: None,
            free_hdd_space: None,
            platform: None,
            total_memory: None,
            version: None,
            write_sect_total: None,
        }
    }
}
