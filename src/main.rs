use crate::routeros::client::api::ApiClient;
use crate::routeros::client::config::{ConfigClient, RosModel};
use crate::routeros::client::{Client, ResourceListAccess};
use crate::routeros::generated::interface::bridge::port::BridgePort;
use crate::routeros::generated::interface::bridge::Bridge;
use crate::routeros::generated::interface::ethernet::switch::egress_vlan_tag::EthernetSwitchEgressVlanTag;
use crate::routeros::generated::interface::ethernet::switch::ingress_vlan_translation::EthernetSwitchIngressVlanTranslation;
use crate::routeros::generated::interface::ethernet::switch::vlan::EthernetSwitchVlan;
use crate::routeros::generated::interface::ethernet::Ethernet;
use crate::routeros::generated::interface::wireless::Wireless;
use crate::routeros::generated::system::identity::Identity;
use crate::routeros::model::{
    FieldDescription, RosFieldAccessor, RosFieldValue, RouterOsResource, ValueFormat,
};
use field_ref::field_ref_of;
use serde::de::Unexpected::Str;
use std::collections::{HashMap, HashSet};
use std::ops::DerefMut;

pub mod routeros;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    //    let x = Ok(Some(true));
    /*
        let conn = ClientBuilder::new()
            .connection_verbose(true)
            .danger_accept_invalid_certs(true)
            .build()?;
    */
    /*
        let client = HttpClient::new(
            conn,
            "10.192.65.249".parse()?,
            "dev-api".into(),
            "bz5g2b11gp".into(),
        );
    */

    let mut client = ApiClient::new(
        "10.192.65.12".parse()?,
        "dev-api".into(),
        "bz5g2b11gp".into(),
    )
    .await?;

    /*
    let mut ports: Vec<BridgePort> = client.list().await?;

    for bp in ports.iter_mut() {
        println!("Bridge: {:?}", bp);
        println!("Modified: {}", bp.is_modified());
        bp.comment.set(Some(String::from("Hello World")));
        println!("Modified: {}", bp.is_modified());
        dump_modifications(bp);
    }*/

    //let name = Some(String::from("loopback"));
    let mut config = ConfigClient::with_default_config(RosModel::Crs109).await?;
    let mut ethernet = client.fetch::<Ethernet>().await?;
    let mut bridge = client.fetch::<Bridge>().await?;
    let mut bridge_port = client.fetch::<BridgePort>().await?;
    let mut switch_egress_vlan = client.fetch::<EthernetSwitchVlan>().await?;
    let mut switch_egress_vlan_tag = client.fetch::<EthernetSwitchEgressVlanTag>().await?;
    let mut switch_vlan_translation = client
        .fetch::<EthernetSwitchIngressVlanTranslation>()
        .await?;

    let loopback_bridge =
        bridge.get_or_create_by_value(&field_ref_of!(Bridge => name), String::from("loopback"));
    loopback_bridge.comment.clear();
    let switch_bridge =
        bridge.get_or_create_by_value(&field_ref_of!(Bridge => name), String::from("switch"));
    switch_bridge.comment.clear();
    bridge_port.put_aside(
        &(|p: &BridgePort| {
            p.bridge
                .get()
                .as_ref()
                .map(|name| "switch" == name)
                .unwrap_or(false)
        }),
    );
    for (dfn_name, curr_name) in [
        ("ether1", "e01-uplink"),
        ("ether2", "e02-notebook"),
        ("ether3", "e03"),
        ("ether4", "e04"),
        ("ether5", "e05"),
        ("ether6", "e06"),
        ("ether7", "e07-phone"),
        ("ether8", "e08"),
        ("sfp1", "s01"),
    ] {
        ethernet
            .get_or_create_by_value(
                &field_ref_of!(Ethernet => default_name),
                String::from(dfn_name),
            )
            .name
            .set(String::from(curr_name));
        let port = bridge_port.get_or_create_by_value(
            &field_ref_of!(BridgePort => interface),
            String::from(curr_name),
        );
        port.bridge.set(String::from("switch"));
        port.ingress_filtering.set(false);
    }

    for vlan in switch_egress_vlan_tag.iter_mut() {
        vlan.tagged_ports.clear();
    }
    switch_egress_vlan_tag.put_all_aside();
    switch_vlan_translation
        .iter_mut()
        .for_each(|t| t.ports.clear());
    switch_vlan_translation.put_all_aside();
    switch_egress_vlan.iter_mut().for_each(|t| t.ports.clear());
    switch_egress_vlan.put_all_aside();

    for (interface, vlan_ids, untagged_vlan) in [
        (
            "e01-uplink",
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            None,
        ),
        ("e02-notebook", vec![], Some(6)),
        ("e03", vec![], Some(1)),
        ("e04", vec![], Some(1)),
        ("e05", vec![], Some(1)),
        ("e06", vec![], Some(1)),
        ("e07-phone", vec![], Some(1)),
        ("e08", vec![], Some(1)),
        ("s01", vec![], Some(1)),
        ("switch1-cpu", vec![], Some(5)),
    ] {
        for vlan_id in vlan_ids {
            switch_egress_vlan_tag
                .get_or_create_by_value(
                    &field_ref_of!(EthernetSwitchEgressVlanTag => vlan_id),
                    vlan_id,
                )
                .tagged_ports
                .get_or_insert(HashSet::new())
                .insert(String::from(interface));
            switch_egress_vlan
                .get_or_create_by_value(&field_ref_of!(EthernetSwitchVlan => vlan_id), vlan_id)
                .ports
                .get_or_insert(HashSet::new())
                .insert(String::from(interface));
        }
        if let Some(vlan_id) = untagged_vlan {
            switch_vlan_translation
                .get_or_create_by_value2(
                    &field_ref_of!(EthernetSwitchIngressVlanTranslation => customer_vid),
                    0,
                    &field_ref_of!(EthernetSwitchIngressVlanTranslation => new_customer_vid),
                    vlan_id,
                )
                .ports
                .get_or_insert(HashSet::new())
                .insert(String::from(interface));
            switch_egress_vlan
                .get_or_create_by_value(&field_ref_of!(EthernetSwitchVlan => vlan_id), vlan_id)
                .ports
                .get_or_insert(HashSet::new())
                .insert(String::from(interface));
        }
    }

    for (vlan_id, interfaces) in [(1, HashSet::from(["e01-uplink"]))] {
        let mut egress = switch_egress_vlan_tag.get_or_create_by_value(
            &field_ref_of!(EthernetSwitchEgressVlanTag => vlan_id),
            vlan_id,
        );
        egress.tagged_ports.set(HashSet::from_iter(
            interfaces.iter().map(|v| String::from(*v)),
        ));
    }

    //println!("Data before: {:?}", data);
    bridge.commit(&mut config).await?;
    ethernet.commit(&mut config).await?;
    bridge_port.commit(&mut config).await?;
    switch_egress_vlan_tag.commit(&mut config).await?;
    switch_vlan_translation.commit(&mut config).await?;
    switch_egress_vlan.commit(&mut config).await?;

    /*
    let wireless = client.fetch::<Wireless>().await?;
    println!("Wireless: {:?}", wireless);

    let mut identity = client.get::<Identity>().await?;
    let value = identity.deref_mut();
    value.name.set(String::from("Hello System"));
    println!("Identity: {:?}", identity);
    identity.commit(&mut config).await?;
    println!("Identity: {:?}", identity);

     */
    println!("Update cmd: \n{}", config.dump_cmd());
    //for row in data.iter() {
    //    println!("Row: {:?}", row);
    //}

    Ok(())
}

/*
fn id2tuple(id: (&FieldDescription, &dyn RosFieldAccessor)) -> (&'static str, String) {
    (id.0.name, id.1.api_value(&ValueFormat::Cli))
}

 */

fn dump_modifications<Resource: RouterOsResource>(resource: &Resource) {
    for modified_entry in resource
        .fields()
        .map(|e| (e.0, e.1.modified_value(&ValueFormat::Api)))
        .filter(|e| e.1.is_some())
    {
        println!("Entry: {}: {:?}", modified_entry.0.name, modified_entry.1);
    }
}
