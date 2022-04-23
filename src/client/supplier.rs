use crate::client::Client;
use crate::RosError;
use async_trait::async_trait;

pub fn single_config_supplier<C: Client>(client: C) -> impl ClientSupplier<C, C> {
    SingleConfigSupplier { client }
}

pub fn split_config_supplier<ReadClient: Client, WriteClient: Client>(
    read_client: ReadClient,
    write_client: WriteClient,
) -> impl ClientSupplier<ReadClient, WriteClient> {
    SplitConfigSupplier {
        read_client,
        write_client,
    }
}

pub trait ClientSupplier<ReadClient: Client, WriteClient: Client> {
    fn read_client(&mut self) -> &mut ReadClient;
    fn write_client(&mut self) -> &mut WriteClient;
}

#[async_trait]
pub trait RouterOsConfiguration: Send {
    async fn apply<CS: ClientSupplier<RC, WC> + Sync + Send, RC: Client, WC: Client>(
        &self,
        client: &mut CS,
    ) -> Result<(), RosError>;
}

pub struct SingleConfigSupplier<C: Client> {
    client: C,
}

impl<C: Client> ClientSupplier<C, C> for SingleConfigSupplier<C> {
    fn read_client(&mut self) -> &mut C {
        &mut self.client
    }

    fn write_client(&mut self) -> &mut C {
        &mut self.client
    }
}

pub struct SplitConfigSupplier<RC: Client, WC: Client> {
    read_client: RC,
    write_client: WC,
}

impl<RC: Client, WC: Client> ClientSupplier<RC, WC> for SplitConfigSupplier<RC, WC> {
    fn read_client(&mut self) -> &mut RC {
        &mut self.read_client
    }

    fn write_client(&mut self) -> &mut WC {
        &mut self.write_client
    }
}
