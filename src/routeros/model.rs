use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fmt::Debug;
use std::net::IpAddr;
use std::str::FromStr;
use tokio::runtime::Builder;

pub mod bridge;
//pub mod generated;
pub mod system;

pub trait ResourceBuilder<R>: Send + Sync
where
    R: RouterOsResource + Sized,
{
    fn write_field<K, V>(&mut self, key: K, value: V) -> bool
    where
        K: AsRef<str>,
        V: AsRef<str>,
        V: ToString;
    fn build(self) -> R;
}

pub trait RouterOsResource: Sized + Debug + Send {
    type Builder: ResourceBuilder<Self>;

    fn resource_path() -> &'static str;
    fn resource_url(ip_addr: IpAddr) -> String {
        format!("https://{}/rest/{}", ip_addr, Self::resource_path())
    }
    fn builder() -> Self::Builder;
}

//pub trait RouterOsSingleResource: RouterOsResource + DeserializeOwned {}
//pub trait RouterOsListResource: RouterOsResource + DeserializeOwned {}

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
