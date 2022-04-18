use crate::client::config::ConfigClient;
use crate::client::Client;
use crate::client::ResourceAccess;
use crate::generated::interface::ethernet::Ethernet;
use crate::generated::interface::wireless::Wireless;
use crate::hardware::SwitchChip::{Qca8513L, _98DX3236};
use crate::RosError;
use field_ref::field_ref_of;
pub enum MikrotikModel {
    Crs109,
    Crs326,
}
pub enum SwitchChip {
    Qca8513L,
    _98DX3236,
}

impl MikrotikModel {
    pub async fn init(&self, client: &mut ConfigClient) -> Result<(), RosError> {
        let mut eth = client.fetch::<Ethernet>().await?;
        let mut wlan = client.fetch::<Wireless>().await?;
        for if_name in self.ethernet_interface_names() {
            eth.get_or_create_by_value(&field_ref_of!(Ethernet => default_name), if_name);
        }
        for if_name in self.wireless_interface_names() {
            wlan.get_or_create_by_value(&field_ref_of!(Wireless => default_name), if_name);
        }

        wlan.commit(client).await?;
        eth.commit(client).await?;
        client.dump_cmd();
        Ok(())
    }
    pub fn switch_chip(&self) -> SwitchChip {
        match self {
            MikrotikModel::Crs109 => Qca8513L,
            MikrotikModel::Crs326 => _98DX3236,
        }
    }
    pub fn ethernet_interface_names(&self) -> Vec<String> {
        match self {
            MikrotikModel::Crs109 => (1..9)
                .map(|idx| format!("ether{}", idx))
                .chain(Some(String::from("sfp1")))
                .collect(),
            MikrotikModel::Crs326 => (1..25)
                .map(|idx| format!("ether{}", idx))
                .chain((1..3).map(|idx| format!("sfp-sfpplus{}", idx)))
                .collect(),
        }
    }
    pub fn wireless_interface_names(&self) -> Vec<String> {
        match self {
            MikrotikModel::Crs109 => {
                vec![String::from("wlan1")]
            }
            MikrotikModel::Crs326 => Vec::new(),
        }
    }
}
