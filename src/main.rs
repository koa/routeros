use crate::routeros::client::api::ApiClient;
use crate::routeros::client::Client;

use crate::routeros::generated::interface::bridge::port::BridgePort;

mod routeros;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Start");

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
        "10.192.65.14".parse()?,
        "dev-api".into(),
        "bz5g2b11gp".into(),
    )
    .await?;

    let ports: Vec<BridgePort> = client.list().await?;
    for bp in ports {
        println!("Bridge port: {:?}", bp);
    }

    //for x in sr {
    //    println!("Resource: {}", x);
    //}

    //let systemResource: SystemResource = client.get().await?;
    //println!("System Resource: {:#?}", systemResource);

    Ok(())
}
