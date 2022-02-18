use crate::routeros::client::Client;
use crate::routeros::model::RouterOsResource;
use async_trait::async_trait;
use std::net::IpAddr;

pub struct HttpClient {
    client: reqwest::Client,
    target: IpAddr,
    username: String,
    password: String,
}

impl HttpClient {
    pub fn new(
        client: reqwest::Client,
        target: IpAddr,
        username: String,
        password: String,
    ) -> HttpClient {
        HttpClient {
            client,
            target,
            username,
            password,
        }
    }
}

/*
#[async_trait]
impl Client<reqwest::Error> for HttpClient {
    async fn list<Resource>(&self) -> Result<Vec<Resource>, reqwest::Error>
    where
        Resource: RouterOsResource,
    {
        self.client
            .get(Resource::resource_url(self.target))
            .basic_auth(self.username.as_str(), Some(self.password.as_str()))
            .send()
            .await?
            .json::<Vec<Resource>>()
            .await
    }

    async fn get<Resource>(&self) -> Result<Resource, reqwest::Error>
    where
        Resource: RouterOsResource,
    {
        self.client
            .get(Resource::resource_url(self.target))
            .basic_auth(self.username.as_str(), Some(self.password.as_str()))
            .send()
            .await?
            .json::<Resource>()
            .await
    }
}
*/
