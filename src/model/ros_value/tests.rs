use std::collections::HashSet;

use crate::model::ros_value::RosValue;

#[test]
fn check_hash_parse() {
    let parsed_value = HashSet::<u16>::from_api("1").ok();
    let mut result = HashSet::new();
    result.insert(1u16);
    assert_eq!(parsed_value, Some(result));
}
