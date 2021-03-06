use std::iter::Chain;
use std::mem;
use std::mem::{swap, take};
use std::ops::{Deref, DerefMut};
use std::slice::{Iter, IterMut};

use async_trait::async_trait;
use field_ref::FieldRef;

use crate::model::RosFieldValue;
use crate::model::RosValue;
use crate::model::RouterOsListResource;
use crate::model::RouterOsResource;
use crate::model::RouterOsSingleResource;
use crate::RosError;

pub mod api;
pub mod config;

pub mod supplier;

#[async_trait]
pub trait Client: Send + Sync {
    async fn fetch<Resource>(&mut self) -> Result<ResourceListAccess<Resource>, RosError>
    where
        Resource: RouterOsListResource,
    {
        let fetched_data: Vec<Resource> =
            self.list()
                .await
                .map_err(|error| RosError::StructureAccessError {
                    structure: Resource::resource_path(),
                    error: Box::new(error),
                })?;
        Ok(ResourceListAccess {
            fetched_data,
            new_data: Vec::new(),
            remove_data: Vec::new(),
            remove_if_not_touched: Vec::new(),
        })
    }
    async fn get<Resource>(&mut self) -> Result<ResourceSingleAccess<Resource>, RosError>
    where
        Resource: RouterOsSingleResource,
    {
        let value = if let Some(value) = self.list().await?.into_iter().next() {
            value
        } else {
            Resource::default()
        };
        Ok(ResourceSingleAccess { data: value })
    }
    async fn list<Resource>(&mut self) -> Result<Vec<Resource>, RosError>
    where
        Resource: RouterOsResource;
    async fn update<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsListResource;
    async fn set<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsSingleResource;
    async fn add<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsListResource;
    async fn delete<Resource>(&mut self, resource: Resource) -> Result<(), RosError>
    where
        Resource: RouterOsListResource;
}

#[derive(Debug, Default)]
pub struct ResourceListAccess<Resource>
where
    Resource: RouterOsListResource,
{
    fetched_data: Vec<Resource>,
    new_data: Vec<Resource>,
    remove_data: Vec<Resource>,
    remove_if_not_touched: Vec<Resource>,
}

#[derive(Debug, Default)]
pub struct ResourceSingleAccess<Resource>
where
    Resource: RouterOsSingleResource,
{
    data: Resource,
}

impl<R: RouterOsSingleResource> Deref for ResourceSingleAccess<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<R: RouterOsSingleResource> DerefMut for ResourceSingleAccess<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[async_trait]
impl<R> ResourceAccess for ResourceSingleAccess<R>
where
    R: RouterOsSingleResource,
{
    async fn commit_remove<'a, C>(&'a mut self, _client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        Ok(())
    }
    async fn commit_update<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        let data = take(&mut self.data);
        //async {
        if data.is_modified() {
            client.set(data).await?;
        }
        //self.rollback(client).await?;
        Ok(())
    }
    async fn commit_add<'a, C>(&'a mut self, _client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        Ok(())
    }
    async fn rollback<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        let fetched_data: Vec<R> = client.list().await?;
        if let Some(existing_value) = fetched_data.into_iter().next() {
            self.data = existing_value;
        } else {
            self.data = R::default();
        };
        Ok(())
    }
}

#[async_trait]
pub trait ResourceAccess {
    async fn commit_remove<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a;
    async fn commit_update<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a;
    async fn commit_add<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a;
    async fn commit<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        self.commit_remove(client).await?;
        self.commit_update(client).await?;
        self.commit_add(client).await?;
        self.rollback(client).await?;
        Ok(())
    }
    async fn rollback<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a;
}

/*
#[async_trait]
impl<I: Iterator<Item = Box<dyn ResourceAccess>>> ResourceAccess for I {
    async fn commit_remove<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        for mut r in self.deref_mut() {
            r.commit_remove(client).await?;
        }
        Ok(())
    }

    async fn commit_update<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        for mut r in self.deref_mut() {
            r.commit_update(client).await?;
        }
        Ok(())
    }

    async fn commit_add<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        for mut r in self.deref_mut() {
            r.commit_add(client).await?;
        }
        Ok(())
    }

    async fn rollback<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        for mut r in self.deref_mut() {
            r.rollback(client).await?;
        }
        Ok(())
    }
}
*/
impl<R> ResourceListAccess<R>
where
    R: RouterOsListResource,
{
    /*pub fn add(&mut self, r: R) {
        self.new_data.push(r);
    }*/
    pub fn remove<F>(&mut self, filter: F)
    where
        F: Fn(&R) -> bool,
    {
        self.new_data.retain(|r| !filter(r));

        let mut fetched_data = Vec::new();
        mem::swap(&mut self.fetched_data, &mut fetched_data);
        let (remove, keep): (Vec<R>, Vec<R>) = fetched_data.into_iter().partition(&filter);
        self.fetched_data = keep;
        self.remove_data.extend(remove);

        let mut fetched_data = Vec::new();
        mem::swap(&mut self.remove_if_not_touched, &mut fetched_data);
        let (remove, keep): (Vec<R>, Vec<R>) = fetched_data.into_iter().partition(&filter);
        self.remove_if_not_touched = keep;
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
        if let Some(found_index) = self.fetched_data.iter().position(&filter) {
            return &mut self.fetched_data[found_index];
        }
        if let Some(found_index) = self.new_data.iter().position(&filter) {
            return &mut self.new_data[found_index];
        }
        if let Some(found_index) = self.remove_if_not_touched.iter().position(&filter) {
            let deleted = self.remove_if_not_touched.remove(found_index);
            self.fetched_data.push(deleted);
            return self.fetched_data.last_mut().unwrap();
        }
        self.new_data.push(R::default());
        let last: Option<&'a mut R> = self.new_data.last_mut();
        last.unwrap()
    }
    pub fn put_all_aside(&mut self) {
        let mut list = Vec::new();
        swap(&mut list, &mut self.fetched_data);
        self.remove_if_not_touched.append(&mut list);
    }
    pub fn put_all_aside_and_mutate<M>(&mut self, mutator: &M)
    where
        M: Fn(&mut R),
    {
        self.put_all_aside();
        for entry in self.remove_if_not_touched.iter_mut() {
            mutator(entry);
        }
    }
    pub fn put_aside<F>(&mut self, filter: &F)
    where
        F: Fn(&R) -> bool,
    {
        let mut list = Vec::new();
        swap(&mut list, &mut self.fetched_data);
        let (mut remove, keep): (Vec<R>, Vec<R>) = list.into_iter().partition(&filter);
        self.fetched_data = keep;
        self.remove_if_not_touched.append(&mut remove);
    }
    pub fn put_aside_and_mutate<F, M>(&mut self, filter: &F, mutator: &M)
    where
        F: Fn(&R) -> bool,
        M: Fn(&mut R),
    {
        self.put_aside(filter);
        for entry in self.remove_if_not_touched.iter_mut() {
            if filter(entry) {
                mutator(entry);
            }
        }
    }

    pub fn get_or_create_by_value<V, IV>(
        &mut self,
        field: &FieldRef<R, RosFieldValue<V>>,
        value: IV,
    ) -> &mut R
    where
        V: RosValue<Type = V>,
        IV: Into<V>,
    {
        let value = value.into();
        let entry =
            self.get_or_default(|b| field.get(b).as_ref().map(|s| s == &value).unwrap_or(false));
        field.get_mut(entry).set(value);
        entry
    }
    pub fn get_or_create_by_value2<V1, V2, IV1, IV2>(
        &mut self,
        field1: &FieldRef<R, RosFieldValue<V1>>,
        value1: IV1,
        field2: &FieldRef<R, RosFieldValue<V2>>,
        value2: IV2,
    ) -> &mut R
    where
        V1: RosValue<Type = V1>,
        IV1: Into<V1>,
        V2: RosValue<Type = V2>,
        IV2: Into<V2>,
    {
        let value1 = value1.into();
        let value2 = value2.into();
        let entry = self.get_or_default(|b| {
            field1
                .get(b)
                .as_ref()
                .map(|s| s == &value1)
                .unwrap_or(false)
                && field2
                    .get(b)
                    .as_ref()
                    .map(|s| s == &value2)
                    .unwrap_or(false)
        });
        field1.get_mut(entry).set(value1);
        field2.get_mut(entry).set(value2);
        entry
    }
    pub fn iter(&self) -> Chain<Iter<'_, R>, Iter<'_, R>> {
        self.fetched_data.iter().chain(self.new_data.iter())
    }
    pub fn iter_mut(&mut self) -> Chain<IterMut<'_, R>, IterMut<'_, R>> {
        self.fetched_data.iter_mut().chain(self.new_data.iter_mut())
    }
    pub fn to_be_deleted_iter(&self) -> Chain<Iter<'_, R>, Iter<'_, R>> {
        self.remove_if_not_touched
            .iter()
            .chain(self.remove_data.iter())
    }
}

#[async_trait]
impl<R: RouterOsListResource> ResourceAccess for ResourceListAccess<R> {
    async fn commit_remove<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        let remove_entries: Vec<R> = self
            .remove_data
            .iter()
            .filter(|e| !e.is_dynamic())
            .map(R::clone)
            .collect();
        let remove_untouched_entries: Vec<R> = self
            .remove_if_not_touched
            .iter()
            .filter(|e| !e.is_dynamic())
            .map(R::clone)
            .collect();
        for remove_entry in remove_entries {
            client.delete(remove_entry).await?;
        }
        for remove_entry in remove_untouched_entries {
            client.delete(remove_entry).await?;
        }
        Ok(())
    }
    async fn commit_update<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        let modified_entries: Vec<R> = self
            .fetched_data
            .iter()
            .filter(|e| (!(*e).is_dynamic()) && (*e).is_modified())
            .map(R::clone)
            .collect();
        for update_entry in modified_entries {
            client.update(update_entry).await?;
        }
        Ok(())
    }
    async fn commit_add<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        let new_entries: Vec<R> = self
            .new_data
            .iter()
            .filter(|e| !e.is_dynamic())
            .map(R::clone)
            .collect();
        for new_entry in new_entries {
            client.add(new_entry).await?;
        }
        Ok(())
    }

    async fn rollback<'a, C>(&'a mut self, client: &'a mut C) -> Result<(), RosError>
    where
        C: Client + 'a,
    {
        let fetched_data: Vec<R> = client.list().await?;
        self.remove_data.clear();
        self.new_data.clear();
        self.remove_if_not_touched.clear();
        self.fetched_data = fetched_data;
        Ok(())
    }
}
