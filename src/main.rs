use crate::routeros::client::api::ApiClient;
use crate::routeros::client::{Client, ResourceAccess};
use crate::routeros::generated::interface::bridge::Bridge;
use crate::routeros::generated::system::resource::Resource;

use crate::routeros::model::RouterOsResource;

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

    let mut system: ResourceAccess<Resource> = client.fetch().await?;
    let system_resource = system.get_or_default(|_| true);
    println!("System resource: {:?}", system_resource);
    let mut bridges: ResourceAccess<Bridge> = client.fetch().await?;

    let string_value = "switch";
    let switch =
        bridges.get_or_default(|b| b.name.as_ref().map(|s| s == string_value).unwrap_or(false));
    switch.name.set(Some(String::from(string_value)));
    switch.comment.set(Some(String::from("Hello Comment 2")));

    println!("Switch: {:?}", switch);
    dump_modifications(switch);
    bridges.commit(&mut client).await?;
    println!("Access: {:?}", bridges);
    //for x in sr {
    //    println!("Resource: {}", x);
    //}

    //let systemResource: SystemResource = client.get().await?;
    //println!("System Resource: {:#?}", systemResource);

    Ok(())
}

fn dump_modifications<Resource: RouterOsResource>(resource: &Resource) {
    for modified_entry in resource
        .fields()
        .map(|e| (e.0, e.1.modified_value()))
        .filter(|e| e.1.is_some())
    {
        println!("Entry: {}: {:?}", modified_entry.0, modified_entry.1);
    }
}
