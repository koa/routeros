use crate::RosFieldValue;
use async_trait::async_trait;
use field_ref::FieldRef;
use std::collections::HashSet;
use std::future::Future;
use std::iter::Chain;
use std::mem;
use std::slice::{Iter, IterMut};

use crate::routeros::model::{RosValue, RouterOsResource};

pub mod api;
pub mod config;

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
            new_data: Vec::new(),
            remove_data: Vec::new(),
        })
    }
    async fn list<Resource>(&mut self) -> Result<Vec<Resource>, Error>
    where
        Resource: RouterOsResource;
    async fn update<Resource>(&mut self, resource: Resource) -> Result<(), Error>
    where
        Resource: RouterOsResource;
    async fn add<Resource>(&mut self, resource: Resource) -> Result<(), Error>
    where
        Resource: RouterOsResource;
    async fn delete<Resource>(&mut self, resource: Resource) -> Result<(), Error>
    where
        Resource: RouterOsResource;
}
#[derive(Debug, Default)]
pub struct ResourceAccess<Resource>
where
    Resource: RouterOsResource,
{
    fetched_data: Vec<Resource>,
    new_data: Vec<Resource>,
    remove_data: Vec<Resource>,
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
        self.remove_data.extend(remove);
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
    pub fn get_or_create_by_value<V>(
        &mut self,
        field: &FieldRef<R, RosFieldValue<V>>,
        value: V,
    ) -> &mut R
    where
        V: RosValue<Type = V>,
    {
        let entry =
            self.get_or_default(|b| field.get(b).as_ref().map(|s| s == &value).unwrap_or(false));
        field.get_mut(entry).set(value);
        entry
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
        let new_entries: Vec<R> = self.new_data.iter().map(R::clone).collect();
        let remove_entries = self.remove_data.clone();
        async {
            for remove_entry in remove_entries {
                client.delete(remove_entry).await?;
            }
            for update_entry in modified_entries {
                client.update(update_entry).await?;
            }
            for new_entry in new_entries {
                client.add(new_entry).await?;
            }
            self.rollback(client).await?;
            Ok(())
        }
    }
    pub fn rollback<'a, C, E>(
        &'a mut self,
        client: &'a mut C,
    ) -> impl Future<Output = Result<(), E>> + 'a
    where
        C: Client<E> + 'a,
        E: std::error::Error,
    {
        async {
            let fetched_data: Vec<R> = client.list().await?;
            self.remove_data.clear();
            self.new_data.clear();
            self.fetched_data = fetched_data;
            Ok(())
        }
    }
    pub fn iter(&self) -> Chain<Iter<'_, R>, Iter<'_, R>> {
        self.fetched_data.iter().chain(self.new_data.iter())
    }
    pub fn iter_mut(&mut self) -> Chain<IterMut<'_, R>, IterMut<'_, R>> {
        self.fetched_data.iter_mut().chain(self.new_data.iter_mut())
    }
}
