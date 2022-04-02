use super::RosFieldValue;
use crate::routeros::client::api::RosError;
use std::num::ParseIntError;
use std::ops::Deref;

#[test]
fn test_string() -> Result<(), RosError> {
    let mut value: RosFieldValue<String> = RosFieldValue::new("hello")?;

    assert_eq!(Some(String::from("hello")), *value.deref());
    assert_eq!(None, value.modified_value());

    value.set(String::from("X2"));
    assert_eq!(Some(String::from("X2")), value.modified_value());
    value.reset();
    assert_eq!(None, value.modified_value());
    Ok(())
}

#[test]
fn test_option_boolean() -> Result<(), RosError> {
    let mut value: RosFieldValue<bool> = RosFieldValue::new("true")?;
    assert_eq!(Some(true), *value.deref());
    value.clear();
    assert_eq!(Some(String::from("")), value.modified_value());
    value.set(false);
    assert_eq!(Some(String::from("false")), value.modified_value());
    Ok(())
}

#[test]
fn test_u16() -> Result<(), RosError> {
    let mut value: RosFieldValue<u16> = RosFieldValue::new("12")?;
    assert_eq!(Some(12), *value.deref());
    value.clear();
    assert_eq!(Some(String::from("")), value.modified_value());
    value.set(13);
    assert_eq!(Some(String::from("13")), value.modified_value());
    Ok(())
}
