use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::future::ready;
use std::mem::take;

use async_trait::async_trait;

use crate::client::Client;
use crate::hardware::MikrotikModel;
use crate::model::{RouterOsListResource, RouterOsResource, RouterOsSingleResource, ValueFormat};
use crate::RosError;

pub struct ConfigClient {
    output: String,
    model: HashMap<&'static str, Vec<HashMap<&'static str, String>>>,
    current_context: &'static str,
}

impl ConfigClient {
    pub fn new() -> ConfigClient {
        ConfigClient {
            output: String::new(),
            model: HashMap::new(),
            current_context: "",
        }
    }
    pub async fn with_default_config(model: MikrotikModel) -> Result<ConfigClient, RosError> {
        let mut ret = Self::new();
        model.init(&mut ret).await?;
        Ok(ret)
    }
    pub fn dump_cmd(&mut self) -> String {
        self.current_context = "";
        take(&mut self.output)
    }
    fn ensure_context(&mut self, resource_path: &'static str) {
        if resource_path != self.current_context {
            self.output.push('/');
            self.output
                .push_str(resource_path.replace('/', " ").as_str());
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
                    .map(|v| (f.0.name, quote_routeros(&v)))
            })
            .for_each(|(key, value)| self.output.push_str(&format!(" {key}={value}")));
    }

    fn values_of_resource<Resource: RouterOsResource>(
        &mut self,
    ) -> &mut Vec<HashMap<&'static str, String>> {
        let values = match self.model.entry(Resource::resource_path()) {
            Entry::Occupied(value) => value.into_mut(),
            Entry::Vacant(v) => v.insert(Vec::new()),
        };
        values
    }

    fn write_resource<Resource>(resource: Resource, found_ref: &mut HashMap<&str, String>)
    where
        Resource: RouterOsResource,
    {
        for (key, value) in resource
            .fields()
            .map(|(description, field)| (description.name, field.api_value(&ValueFormat::Cli)))
        {
            found_ref.insert(key, value);
        }
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
impl Client for ConfigClient {
    async fn list<Resource>(&mut self) -> Result<Vec<Resource>, RosError>
    where
        Resource: RouterOsResource,
    {
        let values = self.values_of_resource::<Resource>();
        let mut stored_data = Vec::new();
        for record in values {
            let mut entry = Resource::default();
            for (description, field) in entry.fields_mut() {
                if let Some(value) = record.get(description.name) {
                    field.set_from_api(value)?;
                }
            }
            stored_data.push(entry);
        }
        ready(Ok(stored_data)).await
    }

    async fn update<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsListResource,
    {
        if let Some((description, field)) = resource.id_field() {
            if resource.is_modified() {
                let key = description.name;
                let api_value = field.api_value(&ValueFormat::Cli);
                let value = quote_routeros(&api_value);
                self.ensure_context(Resource::resource_path());
                self.output
                    .push_str(&format!("set [ find where {key}={value} ] "));
                self.append_modified_fields(&resource);
                self.output.push('\n');

                let values = self.values_of_resource::<Resource>();
                if let Some(found_ref) = values
                    .iter_mut()
                    .find(|r| Some(&api_value) == r.get(description.name))
                {
                    Self::write_resource(resource, found_ref);
                }
            }
        }
        ready(Ok(())).await
    }

    async fn set<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsSingleResource,
    {
        self.ensure_context(Resource::resource_path());
        self.output.push_str(&format!("set"));
        self.append_modified_fields(&resource);
        self.output.push('\n');

        let values = self.values_of_resource::<Resource>();
        if values.is_empty() {
            values.push(HashMap::new());
        }
        Self::write_resource(resource, &mut values[0]);
        Ok(())
        //ready(Ok(())).await
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

            let values = self.values_of_resource::<Resource>();
            let mut data = HashMap::new();
            Self::write_resource(resource, &mut data);
            values.push(data);
        }
        ready(Ok(())).await
    }

    async fn delete<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsResource,
    {
        if let Some((description, field)) = resource.id_field() {
            let key = description.name;
            let value = quote_routeros(&field.api_value(&ValueFormat::Cli));
            self.ensure_context(Resource::resource_path());
            self.output
                .push_str(&format!("remove [find where {key}={value}]\n"));

            let values = self.values_of_resource::<Resource>();
            values.retain(|r| Some(&value) != r.get(description.name));
        }
        ready(Ok(())).await
    }
}
