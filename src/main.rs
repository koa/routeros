use crate::routeros::client::api::ApiClient;
use crate::routeros::client::config::ConfigClient;
use crate::routeros::client::{Client, ResourceAccess};
use crate::routeros::generated::interface::bridge::Bridge;
use crate::routeros::model::{RosFieldValue, RouterOsResource, ValueFormat};
use field_ref::field_ref_of;

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
    let mut data: ResourceAccess<Bridge> = client.fetch().await?;
    //data.remove(|b| b.name.get() == &name);
    let loopback =
        data.get_or_create_by_value(&field_ref_of!(Bridge => name), String::from("loopback"));
    loopback.comment.set(String::from("Kommentar 2"));
    //loopback.comment.clear();
    loopback.disabled.set(false);

    let mut config = ConfigClient::new();

    data.commit(&mut config).await?;
    println!("Update cmd: \n{}", config.to_string());
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
        println!("Entry: {}: {:?}", modified_entry.0, modified_entry.1);
    }
}
