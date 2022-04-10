use crate::routeros::client::api::ApiClient;
use crate::routeros::client::config::{ConfigClient, RosModel};
use crate::routeros::client::{Client, ResourceAccess};
use crate::routeros::generated::interface::bridge::Bridge;
use crate::routeros::generated::interface::ethernet::switch::ingress_vlan_translation::EthernetSwitchIngressVlanTranslation;
use crate::routeros::generated::interface::ethernet::switch::vlan::EthernetSwitchVlan;
use crate::routeros::generated::interface::ethernet::Ethernet;
use crate::routeros::model::{RosFieldValue, RouterOsResource, ValueFormat};
use field_ref::field_ref_of;
use serde::de::Unexpected::Str;

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
    let mut data: ResourceAccess<Ethernet> = config.fetch().await?;

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
        data.get_or_create_by_value(
            &field_ref_of!(Ethernet => default_name),
            String::from(dfn_name),
        )
        .name
        .set(String::from(curr_name));
    }

    //println!("Data before: {:?}", data);
    data.commit(&mut config).await?;
    //println!("Data after: {:?}", data);
    println!("Update cmd: \n{}", config.dump_cmd());
    //for row in data.iter() {
    //    println!("Row: {:?}", row);
    //}

    Ok(())
}

fn dump_modifications<Resource: RouterOsResource>(resource: &Resource) {
    for modified_entry in resource
        .fields()
        .map(|e| (e.0, e.1.modified_value(&ValueFormat::Api)))
        .filter(|e| e.1.is_some())
    {
        println!("Entry: {}: {:?}", modified_entry.0.name, modified_entry.1);
    }
}
