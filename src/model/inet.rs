use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

use crate::model::{RosValue, ValueFormat};
use crate::RosError;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct IpNetAddr {
    ip: IpAddr,
    netmask: u8,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct VlanIpNetAddr {
    vlan_id: u16,
    ip: IpAddr,
    netmask: u8,
}

impl VlanIpNetAddr {
    pub fn new(vlan_id: u16, ip: IpNetAddr) -> Self {
        Self {
            vlan_id,
            ip: ip.ip,
            netmask: ip.netmask,
        }
    }
}

impl Default for VlanIpNetAddr {
    fn default() -> Self {
        VlanIpNetAddr {
            vlan_id: 0,
            ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            netmask: 0,
        }
    }
}

impl Into<IpNetAddr> for VlanIpNetAddr {
    fn into(self) -> IpNetAddr {
        IpNetAddr {
            ip: self.ip,
            netmask: self.netmask,
        }
    }
}

impl IpNetAddr {
    pub fn new(ip: IpAddr, netmask: u8) -> IpNetAddr {
        IpNetAddr { ip, netmask }
    }
    pub fn ip(&self) -> &IpAddr {
        &self.ip
    }
    pub fn netmask(&self) -> u8 {
        self.netmask
    }
}

impl Default for IpNetAddr {
    fn default() -> Self {
        IpNetAddr {
            ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            netmask: 0,
        }
    }
}

impl FromStr for IpNetAddr {
    type Err = RosError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_api(s)
    }
}

impl RosValue for IpNetAddr {
    type Type = IpNetAddr;
    type Err = RosError;

    fn from_api(value: &str) -> Result<Self::Type, Self::Err> {
        let mut split = value.split('/');
        if let Some(ip_addr_string) = split.next() {
            let ip = IpAddr::from_api(ip_addr_string)?;
            let netmask: u8 = if let Some(netmask) = split.next() {
                netmask.parse()?
            } else {
                match ip {
                    IpAddr::V4(_) => 32,
                    IpAddr::V6(_) => 128,
                }
            };
            if split.next().is_some() {
                Result::Err(RosError::SimpleMessage(format!(
                    "Network address has more than 1 '/': {value}"
                )))
            } else {
                Result::Ok(IpNetAddr { ip, netmask })
            }
        } else {
            Result::Err(RosError::SimpleMessage(format!(
                "Cannot split network address: {value}"
            )))
        }
    }

    fn to_api(&self, _: &ValueFormat) -> String {
        format!(
            "{address}/{netmask}",
            address = self.ip,
            netmask = self.netmask
        )
    }
}
