use std::collections::HashSet;
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use crate::routeros::json::{
    deserialize_number_ranges_from_string, deserialize_optional_from_string,
    serialize_number_ranges_to_string, serialize_optional_to_string, serialize_stringset_to_string,
    stringset_from_string,
};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BridgePortRole {
    DesignatedPort,
    DisabledPort,
    RootPort,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BridgePortFrameType {
    AdmitOnlyUntaggedAndPriorityTagged,
    AdmitOnlyVlanTagged,
}

macro_rules! ros_field {
    ($id:ident:$type:ty) => {};
}

macro_rules! ros_struct {
    (struct $name:ident {$($id:ident:$type:ty)*}) => {
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "kebab-case")]
        struct $name {
            x: u8,
            $(
                #[serde(
                    deserialize_with = "deserialize_optional_from_string",
                    serialize_with = "serialize_optional_to_string",
                    default
                )]
                $id:$type,
            )*
        }
    }; /*
       ($($t:tt)*) => {
           //ros_struct!(($($t)*));
           strip_attrs_pub!(ros_struct!($($t)*));
       };*/
}

macro_rules! impl_extra {
    ( @ $name:ident { } -> ($($result:tt)*) ) => (
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "kebab-case")]
        pub struct $name {
            $($result)*
        }
    );

    ( @ $name:ident { $param:ident : Option<$type:ty>, $($rest:tt)* } -> ($($result:tt)*) ) => (
        impl_extra!(@ $name { $($rest)* } -> (
            $($result)*
               #[serde(
                    deserialize_with = "deserialize_optional_from_string",
                    serialize_with = "serialize_optional_to_string",
                    default
                )]
            pub $param : Option<$type>,
        ));
    );

    ( @ $name:ident { $param:ident : Vec<$type:ty>, $($rest:tt)* } -> ($($result:tt)*) ) => (
        impl_extra!(@ $name { $($rest)* } -> (
            $($result)*
            #[serde(skip_serializing_if = "Vec::is_empty")]
            pub $param : Vec<$type>,
        ));
    );

    ( @ $name:ident { $param:ident : bool, $($rest:tt)* } -> ($($result:tt)*) ) => (
        impl_extra!(@ $name { $($rest)* } -> (
            $($result)*
            #[serde(skip_serializing_if = "bool::not")]
            pub $param : bool,
        ));
    );

    ( $name:ident { $( $param:ident : $($type:tt)* ),* $(,)* } ) => (
        impl_extra!(@ $name { $($param : $($type)*,)* } -> ());
    );
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BridgePort {
    #[serde(rename = ".id")]
    id: String,
    bridge: String,
    interface: String,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    hw: Option<bool>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    disabled: Option<bool>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    port_number: Option<u16>,
    frame_types: Option<BridgePortFrameType>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    bpdu_guard: Option<bool>,
    role: Option<BridgePortRole>,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    pvid: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BridgeVlan {
    #[serde(rename = ".id")]
    id: String,
    bridge: String,
    #[serde(
        deserialize_with = "deserialize_optional_from_string",
        serialize_with = "serialize_optional_to_string",
        default
    )]
    disabled: Option<bool>,
    #[serde(
        deserialize_with = "stringset_from_string",
        serialize_with = "serialize_stringset_to_string",
        default
    )]
    current_tagged: HashSet<String>,
    #[serde(
        deserialize_with = "stringset_from_string",
        serialize_with = "serialize_stringset_to_string",
        default
    )]
    current_untagged: HashSet<String>,
    #[serde(
        deserialize_with = "stringset_from_string",
        serialize_with = "serialize_stringset_to_string",
        default
    )]
    tagged: HashSet<String>,
    #[serde(
        deserialize_with = "stringset_from_string",
        serialize_with = "serialize_stringset_to_string",
        default
    )]
    untagged: HashSet<String>,
    #[serde(
        deserialize_with = "deserialize_number_ranges_from_string",
        serialize_with = "serialize_number_ranges_to_string",
        default
    )]
    vlan_ids: Vec<RangeInclusive<u16>>,
}
