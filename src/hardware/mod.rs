use std::ops::DerefMut;

use field_ref::field_ref_of;

use crate::client::config::ConfigClient;
use crate::client::Client;
use crate::client::ResourceAccess;
use crate::generated::interface::ethernet::Ethernet;
use crate::generated::interface::wireless::Wireless;
use crate::generated::system::resource::Resource;
use crate::hardware::SwitchChip::{Qca8513L, _98DX3236};
use crate::RosError;

pub enum MikrotikModel {
    Crs109,
    Crs326,
}

pub enum SwitchChip {
    Qca8513L,
    _98DX3236,
}

impl MikrotikModel {
    pub fn parse_system(
        resource: &crate::generated::system::resource::Resource,
    ) -> Result<MikrotikModel, RosError> {
        let board = resource
            .board_name
            .get()
            .as_ref()
            .map(String::as_str)
            .unwrap_or("");
        Self::parse_board_name(board).ok_or(RosError::SimpleMessage(format!(
            "Unsupported Board: {board}"
        )))
    }

    pub fn parse_board_name(board: &str) -> Option<MikrotikModel> {
        if board.starts_with("CRS109") {
            Some(MikrotikModel::Crs109)
        } else if board.starts_with("CRS326") {
            Some(MikrotikModel::Crs326)
        } else {
            None
        }
    }
    pub async fn init(&self, client: &mut ConfigClient) -> Result<(), RosError> {
        let mut eth = client.fetch::<Ethernet>().await?;
        let mut wlan = client.fetch::<Wireless>().await?;
        let mut resource = client.get::<Resource>().await?;
        for if_name in self.ethernet_interface_names() {
            eth.get_or_create_by_value(&field_ref_of!(Ethernet => default_name), if_name);
        }
        for if_name in self.wireless_interface_names() {
            wlan.get_or_create_by_value(&field_ref_of!(Wireless => default_name), if_name);
        }

        resource.deref_mut().board_name.set(self.board_name());

        wlan.commit(client).await?;
        eth.commit(client).await?;
        resource.commit(client).await?;
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
    pub fn board_name(&self) -> &'static str {
        match self {
            MikrotikModel::Crs109 => "CRS109 Dummy",
            MikrotikModel::Crs326 => "CRS326 Dummy",
        }
    }
}
