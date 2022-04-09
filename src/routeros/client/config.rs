use crate::routeros::client::api::RosError;
use crate::routeros::model::ValueFormat;
use crate::{Client, RouterOsResource};
use async_trait::async_trait;
use std::future::ready;

pub struct ConfigClient {
    output: String,
    current_context: &'static str,
}

impl ConfigClient {
    pub fn new() -> ConfigClient {
        ConfigClient {
            output: String::new(),
            current_context: "",
        }
    }
    fn ensure_context(&mut self, resource_path: &'static str) {
        if resource_path != self.current_context {
            self.output.push('/');
            self.output.push_str(resource_path);
            self.output.push('\n');
            self.current_context = resource_path;
        }
    }
    fn append_modified_fields<Resource>(&mut self, resource: &Resource)
    where
        Resource: RouterOsResource,
    {
        resource
            .fields()
            .filter_map(|f| {
                f.1.modified_value(&ValueFormat::Cli)
                    .map(|v| (f.0, quote_routeros(&v)))
            })
            .for_each(|(key, value)| self.output.push_str(&format!(" {key}={value}")));
    }
}
fn quote_routeros(value: &str) -> String {
    let mut ret = String::with_capacity(value.len() + 2);
    ret.push('"');
    for ch in value.chars() {
        match ch {
            '"' => ret.push_str("\\\""),
            '\n' => ret.push_str("\\n"),
            '\t' => ret.push_str("\\t"),
            '$' => ret.push_str("\\$"),
            ch => ret.push(ch),
        }
    }
    ret.push('"');
    ret
}

impl ToString for ConfigClient {
    fn to_string(&self) -> String {
        self.output.clone()
    }
}

#[async_trait]
impl Client<RosError> for ConfigClient {
    async fn list<Resource>(&mut self) -> Result<Vec<Resource>, RosError>
    where
        Resource: RouterOsResource,
    {
        ready(Ok(Vec::new())).await
    }

    async fn update<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsResource,
    {
        if let (Some(id_field), Some(id_value)) = (Resource::id_field(), resource.id_value()) {
            if resource.is_modified() {
                self.ensure_context(Resource::resource_path());
                self.output.push_str("set");
                self.append_modified_fields(&resource);
                self.output
                    .push_str(&format!(" [find where {id_field}={id_value}]\n"));
            }
        }
        ready(Ok(())).await
    }

    async fn add<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsResource,
    {
        if resource.is_modified() {
            self.ensure_context(Resource::resource_path());
            self.output.push_str("add");
            self.append_modified_fields(&resource);
            self.output.push_str("\n");
        }
        ready(Ok(())).await
    }

    async fn delete<Resource>(&mut self, key: &str) -> Result<(), RosError>
    where
        Resource: RouterOsResource,
    {
        self.ensure_context(Resource::resource_path());
        self.output
            .push_str(&format!("remove [find where .id={key}]\n"));
        ready(Ok(())).await
    }
}
