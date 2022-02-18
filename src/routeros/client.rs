use async_trait::async_trait;

pub use http::HttpClient;

use crate::routeros::model::RouterOsResource;

pub mod api;
pub mod http;

#[async_trait]
pub trait Client<Error>
where
    Error: std::error::Error,
{
    async fn list<Resource>(&mut self) -> Result<Vec<Resource>, Error>
    where
        Resource: RouterOsResource + Send;

    async fn get<Resource>(&self) -> Result<Resource, Error>
    where
        Resource: RouterOsResource;
}
