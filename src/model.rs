use std::fmt::{Debug, Display, Formatter};
use std::net::IpAddr;
use std::ops::{Deref, DerefMut};

pub use crate::model::ros_value::{RosFieldAccessor, RosValue, ValueFormat};
use crate::RosError;

pub mod inet;

pub mod ros_value;

#[derive(Debug, Clone)]
pub struct RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    original_value: String,
    current_value: Option<T>,
}

impl<T> Default for RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    fn default() -> Self {
        Self {
            original_value: String::new(),
            current_value: Option::None,
        }
    }
}

impl<T> Display for RosFieldValue<T>
where
    T: RosValue<Type = T>,
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(value) = self.current_value.as_ref() {
            value.fmt(f)
        } else {
            f.write_str("<empty>")
            //Ok(())
        }
    }
}

impl<T> RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    pub fn get(&self) -> &Option<T> {
        &self.current_value
    }
    pub fn set<IT>(&mut self, value: IT)
    where
        IT: Into<T>,
    {
        self.current_value = Some(value.into());
    }
    pub fn clear(&mut self) {
        self.current_value = None;
    }
    pub fn original_value(&self) -> String {
        self.original_value.clone()
    }
    fn original_value_converted(&self) -> Option<T> {
        if self.original_value.is_empty() {
            Option::None
        } else {
            match T::from_api(&self.original_value) {
                Ok(value) => Option::Some(value),
                Err(_) => Option::None,
            }
        }
    }
}

impl<T> RosFieldAccessor for RosFieldValue<T>
where
    T: RosValue<Type = T> + Eq,
    // Err: Into<RosError>,
{
    fn modified_value(&self, format: &ValueFormat) -> Option<String> {
        let original_value = self.original_value_converted();
        if original_value == self.current_value {
            return Option::None;
        }
        let new_value = self.api_value(format);
        if new_value.ne(self.original_value.as_str()) {
            Some(new_value)
        } else {
            None
        }
    }

    fn original_value(&self, format: &ValueFormat) -> Option<String> {
        self.original_value_converted().map(|v| v.to_api(format))
    }

    fn api_value(&self, format: &ValueFormat) -> String {
        self.current_value
            .as_ref()
            .map(|v| v.to_api(format))
            .unwrap_or_else(|| String::from(""))
    }

    fn set_from_api(&mut self, value: &str) -> Result<(), RosError> {
        self.original_value = value.to_string();
        self.reset()
    }

    fn clear(&mut self) -> Result<(), RosError> {
        self.current_value = None;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), RosError> {
        if self.original_value.is_empty() {
            self.current_value = None;
        } else {
            self.current_value =
                Some(T::from_api(self.original_value.as_str()).map_err(|e| e.into())?);
        }
        Ok(())
    }

    fn has_value(&self) -> bool {
        self.current_value.is_some()
    }
}

impl<T> Deref for RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    type Target = Option<T::Type>;
    fn deref(&self) -> &Self::Target {
        &self.current_value
    }
}

impl<T> DerefMut for RosFieldValue<T>
where
    T: RosValue<Type = T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current_value
    }
}

pub trait ResourceBuilder<R>: Send + Sync
where
    R: RouterOsResource + Sized,
{
    fn write_field<K, V>(&mut self, key: K, value: V) -> Result<(), RosError>
    where
        K: AsRef<str>,
        V: AsRef<str>,
        V: ToString;
    fn build(self) -> R;
}

#[derive(PartialEq)]
pub struct FieldDescription {
    pub name: &'static str,
    pub is_read_only: bool,
    pub is_id: bool,
}

pub trait RouterOsApiFieldAccess {
    fn fields_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = (&'static FieldDescription, &mut dyn RosFieldAccessor)> + '_>;
    fn fields(
        &self,
    ) -> Box<dyn Iterator<Item = (&'static FieldDescription, &dyn RosFieldAccessor)> + '_>;
    fn is_dynamic(&self) -> bool {
        false
    }
}

pub trait RouterOsResource:
    Sized + Debug + Send + Default + RouterOsApiFieldAccess + Clone
{
    fn resource_path() -> &'static str;

    fn resource_url(ip_addr: IpAddr) -> String {
        format!("https://{}/rest/{}", ip_addr, Self::resource_path())
    }
    fn is_modified(&self) -> bool {
        return self
            .fields()
            .any(|e| e.1.modified_value(&ValueFormat::Api).is_some());
    }
    fn id_field(&self) -> Option<(&'static FieldDescription, &dyn RosFieldAccessor)> {
        self.fields()
            .filter(|(description, value)| description.is_id && value.has_value())
            .next()
    }
}

pub trait RouterOsListResource: RouterOsResource {}

pub trait RouterOsSingleResource: RouterOsResource {}
