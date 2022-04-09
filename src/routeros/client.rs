use async_trait::async_trait;
use std::collections::HashSet;
use std::future::Future;
use std::mem;

use crate::routeros::model::RouterOsResource;

pub mod api;

#[async_trait]
pub trait Client<Error>
where
    Error: std::error::Error,
{
    async fn fetch<Resource>(&mut self) -> Result<ResourceAccess<Resource>, Error>
    where
        Resource: RouterOsResource,
    {
        let fetched_data: Vec<Resource> = self.list().await?;
        Ok(ResourceAccess {
            fetched_data,
            new_data: vec![],
            remove_data: HashSet::new(),
        })
    }
    async fn list<Resource>(&mut self) -> Result<Vec<Resource>, Error>
    where
        Resource: RouterOsResource;
    async fn update<Resource>(&mut self, resource: Resource) -> Result<(), Error>
    where
        Resource: RouterOsResource;

    async fn get<Resource>(&self) -> Result<Resource, Error>
    where
        Resource: RouterOsResource;
}
#[derive(Debug)]
pub struct ResourceAccess<Resource>
where
    Resource: RouterOsResource,
{
    fetched_data: Vec<Resource>,
    new_data: Vec<Resource>,
    remove_data: HashSet<String>,
}

// unsafe impl<R> Send for ResourceAccess<R> where R: RouterOsResource {}

impl<R> ResourceAccess<R>
where
    R: RouterOsResource,
{
    pub fn add(&mut self, r: R) {
        self.new_data.push(r);
    }
    pub fn remove<P>(&mut self, filter: P)
    where
        P: Fn(&R) -> bool,
    {
        self.new_data.retain(|r| !filter(r));
        let mut fetched_data = Vec::new();
        mem::swap(&mut self.fetched_data, &mut fetched_data);
        let (remove, keep): (Vec<R>, Vec<R>) = fetched_data.into_iter().partition(filter);
        self.fetched_data = keep;
        for rem in remove {
            let found_id_field = rem.id_value();
            if let Some(key) = found_id_field {
                self.remove_data.insert(key);
            }
        }
    }
    pub fn find_mut<P>(&mut self, filter: P) -> Vec<&mut R>
    where
        P: Fn(&R) -> bool,
    {
        let mut ret: Vec<&mut R> = Vec::new();
        for entry in self.fetched_data.iter_mut() {
            if filter(entry) {
                ret.push(entry);
            }
        }
        for entry in self.new_data.iter_mut() {
            if filter(entry) {
                ret.push(entry);
            }
        }
        ret
    }
    pub fn get_or_default<'a, F>(&'a mut self, filter: F) -> &'a mut R
    where
        F: Fn(&R) -> bool,
    {
        for entry in self.fetched_data.iter_mut() {
            if filter(entry) {
                return entry;
            }
        }
        if let Some(found_index) = self.new_data.iter().position(filter) {
            return &mut self.new_data[found_index];
        }
        self.new_data.push(R::default());
        let last: Option<&'a mut R> = self.new_data.last_mut();
        last.unwrap()
    }
    pub fn commit<'a, C, E>(
        &'a mut self,
        client: &'a mut C,
    ) -> impl Future<Output = Result<(), E>> + 'a
    where
        C: Client<E> + 'a,
        E: std::error::Error,
    {
        let modified_entries: Vec<R> = self
            .fetched_data
            .iter()
            .filter(|e| (*e).is_modified())
            .map(|e| e.clone())
            .collect();
        async {
            for update_entry in modified_entries {
                let x = client.update(update_entry);
                x.await?;
            }
            Ok(())
        }
    }
}
