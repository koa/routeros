use ipnet::IpNet;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct VlanIpNetAddr {
    vlan_id: u16,
    ip: IpNet,
}

impl VlanIpNetAddr {
    pub fn new(vlan_id: u16, ip: IpNet) -> Self {
        Self { vlan_id, ip }
    }
}

impl Default for VlanIpNetAddr {
    fn default() -> Self {
        VlanIpNetAddr {
            vlan_id: 0,
            ip: IpNet::default(),
        }
    }
}

impl Into<IpNet> for VlanIpNetAddr {
    fn into(self) -> IpNet {
        self.ip
    }
}
