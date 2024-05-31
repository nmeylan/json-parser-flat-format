use std::cmp::Ordering;
use std::hash::BuildHasherDefault;
use std::str::FromStr;
use std::time::Instant;
use crate::{FlatJsonValue, ValueType};

#[cfg(feature = "indexmap")]
type Map<K, V> = indexmap::IndexMap<K, V>;
#[cfg(not(feature = "indexmap"))]
type Map<K, V> = std::collections::HashMap<K, V>;

#[derive(Debug)]
pub enum Value<'a> {
    Object(Map<String, Value<'a >>),
    ObjectSerialized(&'a str),
    Array(Vec<Value<'a >>),
    ArraySerialized(&'a str),
    Number(f64),
    String(&'a str),
    Bool(bool),
    Null,
}

#[macro_export]
macro_rules! vec_matches {
    () => {false};
    ($v1:expr, $v2:expr) => {
        if $v1.len() != $v2.len() {
            false
        } else {
            let mut matches = true;
            for i in 0..$v1.len() {
                if !$v1[i].eq(&$v2[i]) {
                    matches = false;
                    break;
                }
            }
            matches
        }
    }
}



pub fn serialize_to_json(mut data: FlatJsonValue) -> Value {
    let mut root = Value::Object(new_map());
    let mut root_array = Value::Array(Vec::with_capacity(128));

    let mut root_is_obj = true;

    let mut sorted_data = data;
    let start = Instant::now();
    sorted_data.sort_unstable_by(|(a, _), (b, _)|
        // deepest values will go first, because we will iterate in reverse order from the array to pop value
        match b.depth.cmp(&a.depth) {
            Ordering::Equal => b.position.cmp(&a.position),
            cmp => cmp,
        }
    );
    println!("Sort took {}ms", start.elapsed().as_millis());

    let mut current_parent = &mut root;
    let mut previous_parent_pointer: Vec<String> = Vec::with_capacity(10);
    for i in 0..sorted_data.len() {
        let (key, value) = sorted_data.pop().unwrap();

        if key.pointer == "" && matches!(key.value_type, ValueType::Array(_)) {
            root_is_obj = false;
            current_parent = &mut root_array;
            continue;
        }

        if key.depth == 1 {
            match current_parent {
                Value::Object(obj) => {
                    match key.value_type {
                        ValueType::Object => { obj.insert(key.pointer[1..].to_owned(), Value::Object(new_map())); }
                        ValueType::Array(len) => {
                            if let Some(value) = value {
                                obj.insert(key.pointer[1..].to_owned(), Value::ArraySerialized(value));
                            } else {
                                obj.insert(key.pointer[1..].to_owned(), Value::Array(Vec::with_capacity(len)));
                            }
                        }
                        _ => { obj.insert(key.pointer[1..].to_owned(), value_to_json(value, &key.value_type)); }
                    }
                }
                Value::Array(array) => {
                    match key.value_type {
                        ValueType::Object => { array.push(Value::Object(new_map())); }
                        ValueType::Array(len) => {
                            if let Some(value) = value {
                                array.push(Value::ArraySerialized(value));
                            } else {
                                array.push(Value::Array(Vec::with_capacity(len)));
                            }
                        }
                        _ => { array.push(value_to_json(value, &key.value_type)); }
                    }
                }
                _ => panic!("only Object is accepted for root node")
            }
        } else {
            let split = key.pointer.split('/');
            let key_pointer_iter = split.filter(|s| !s.is_empty());
            let key_pointer_len = key_pointer_iter.clone().count();

            let mut should_update_current_parent = true;
            if key_pointer_len > 0 {
                should_update_current_parent = if previous_parent_pointer.len() != key_pointer_len - 1 {
                    true
                } else {
                    let mut matches = true;
                    for (i, s) in key_pointer_iter.clone().enumerate() {
                        if i == key_pointer_len - 1 {
                            break;
                        }
                        if !previous_parent_pointer[i].eq(s) {
                            matches = false;
                            break;
                        }
                    }
                    !matches
                };
            }

            if should_update_current_parent {
                previous_parent_pointer.clear();
                if key_pointer_len > 0 {
                    for (i, s) in key_pointer_iter.clone().enumerate() {
                        if i == key_pointer_len - 1 {
                            break;
                        }
                        previous_parent_pointer.push(s.to_owned());
                    }
                }
                let b = &key.pointer.as_bytes()[1];
                if *b >= 0x30 && *b <= 0x39 {
                    current_parent = &mut root_array;
                } else {
                    current_parent = &mut root;
                }
                for (i, s) in key_pointer_iter.clone().enumerate() {
                    if i == key_pointer_len - 1 {
                        break;
                    }
                    match current_parent {
                        Value::Object(ref mut obj) => {
                            current_parent = obj.get_mut(s)
                                .unwrap();
                            // .expect(format!("Expected to find parent for {}, current segment {}", key.pointer, s).as_str());
                        }
                        Value::Array(ref mut array) => {
                            current_parent = array.get_mut(usize::from_str(s).unwrap())
                                .unwrap();
                            // .expect(format!("Expected to find parent at index for {}, current segment {}", key.pointer, s).as_str());
                        }
                        _ => panic!("only Object is accepted for root node")
                    }
                }
            }

            let k = key_pointer_iter.last().unwrap();
            match current_parent {
                Value::Object(obj) => {
                    match key.value_type {
                        ValueType::Object => { obj.insert(k.to_owned(), Value::Object(new_map())); }
                        ValueType::Array(len) => {
                            if let Some(value) = value {
                                obj.insert(k.to_owned(), Value::ArraySerialized(value));
                            } else {
                                obj.insert(k.to_owned(), Value::Array(Vec::with_capacity(len)));
                            }
                        }
                        _ => { obj.insert(k.to_owned(), value_to_json(value, &key.value_type)); }
                    }
                }
                Value::Array(array) => {
                    match key.value_type {
                        ValueType::Object => { array.push(Value::Object(new_map())); }
                        ValueType::Array(len) => {
                            if let Some(value) = value {
                                array.push(Value::ArraySerialized(value));
                            } else {
                                array.push(Value::Array(Vec::with_capacity(len)));
                            }
                        }
                        _ => { array.push(value_to_json(value, &key.value_type)); }
                    }
                }
                _ => panic!("only Object is accepted for root node")
            }
        }
    }

    if root_is_obj {
        root
    } else {
        root_array
    }
}

#[inline]
fn new_map<'a>() -> Map<String, Value<'a>> {
    #[cfg(feature = "indexmap")]{
        indexmap::IndexMap::new()
    }
    #[cfg(not(feature = "indexmap"))]{
        std::collections::HashMap::new()
    }
}

// Helper function to convert string values to JSON values based on ValueType
fn value_to_json<'a>(value: Option<&'a str>, value_type: &ValueType) -> Value<'a> {
    if let Some(value) = value {
        match value_type {
            ValueType::Number => value.parse::<f64>().map(Value::Number).unwrap_or(Value::Null),
            ValueType::String => Value::String(value),
            ValueType::Bool => Value::Bool(value == "true" || value == "1"),
            ValueType::Null => Value::Null,
            _ => Value::Null, // this should not happen as arrays and objects are handled separately
        }
    } else {
        Value::Null
    }
}

impl <'a>Value<'a> {
    pub fn to_json(&self) -> String {
        self._to_json(1)
    }
    fn _to_json(&self, depth: usize) -> String {
        match self {
            Value::Object(obj) => {
                let members: Vec<String> = obj.iter().map(|(k, v)| format!("{:indent$}\"{}\": {}", "", k, v._to_json(depth + 1), indent = depth * 2)).collect();
                format!("{{\n{}\n{:indent$}}}", members.join(",\n"), "", indent = (depth - 1) * 2)
            }
            Value::Array(arr) => {
                let mut contains_nested_array = false;
                let elements: Vec<String> = arr.iter().map(|v| {
                    if matches!(v, Value::Array(_)) || matches!(v, Value::Object(_)) {
                        contains_nested_array = true;
                        format!("{:indent$}{}", "", v._to_json(depth + 1), indent = (depth) * 2)
                    } else {
                        v._to_json(depth)
                    }
                }).collect();
                if contains_nested_array {
                    format!("[\n{}\n{:indent$}]", elements.join(",\n"), "", indent = (depth - 1) * 2)
                } else {
                    format!("[{}]", elements.join(", "))
                }
            }
            Value::Number(num) => num.to_string(),
            Value::String(s) => format!("\"{}\"", s.replace("\"", "\\\"")),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::ArraySerialized(value) => value.to_string(),
            _ => panic!("todo")
        }
    }
}

#[cfg(test)]
#[cfg(feature = "indexmap")] // to ease testing we use indexmap to have deterministic output
mod tests {
    use crate::{JSONParser, ParseOptions};
    use crate::serializer::serialize_to_json;

    #[test]
    fn nested_object() {
        let json =
            r#"{
  "id": 1,
  "maxLevel": 99,
  "name": "NV_BASIC",
  "aaa": true,
  "bbb": null,
  "flags": {
    "a": true,
    "b": false,
    "c": {
      "nested": "Oui"
    }
  }
}"#;

        let vec = JSONParser::parse(json,ParseOptions::default()).unwrap().json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }
    #[test]
    fn lifetime_test() {
        let result = {
            let json =
                r#"{
  "id": 1,
  "maxLevel": 99,
  "name": "NV_BASIC",
  "aaa": true,
  "bbb": null,
  "flags": {
    "a": true,
    "b": false,
    "c": {
      "nested": "Oui"
    }
  }
}"#;

             JSONParser::parse(json,ParseOptions::default()).unwrap()
        };
        let mut vec = result.json;
        vec[0].1 = Some("12");
        let value = serialize_to_json(vec);
        println!("{:?}", value.to_json());
    }

    #[test]
    fn simple_array() {
        let json =
            r#"[1, 2, 3]"#;

        let res = JSONParser::parse(json,ParseOptions::default()).unwrap();
        let vec = res.json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }

    #[test]
    fn array_of_object() {
        let json =
            r#"[
  {
    "id": 1,
    "maxLevel": 99,
    "name": "NV_BASIC",
    "aaa": true,
    "bbb": null,
    "flags": {
      "a": true,
      "b": false,
      "c": {
        "nested": "Oui"
      }
    }
  },
  {
    "id": 2,
    "maxLevel": 10,
    "name": "BASH",
    "flags": {
      "a": true,
      "b": false,
      "c": {
        "nested": "Oui"
      }
    }
  }
]"#;

        let res = JSONParser::parse(json,ParseOptions::default()).unwrap();
        let vec = res.json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }

    #[test]
    fn array_of_array() {
        let json =
            r#"[
  [1, 2, 3],
  [6, 7, 8]
]"#;

        let res = JSONParser::parse(json,ParseOptions::default()).unwrap();
        let vec = res.json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }

    #[test]
    fn actual_test_data() {
        let json =
            r#"{
  "skills": [
    {
      "description": "Basic Skill",
      "id": 1,
      "maxLevel": 9,
      "name": "NV_BASIC",
      "basicSkillPerLevel": [
        {
          "level": 1,
          "value": "Trade"
        },
        {
          "level": 2,
          "value": "Emoticon"
        },
        {
          "level": 3,
          "value": "Sit"
        },
        {
          "level": 4,
          "value": "Chat Room (create)"
        },
        {
          "level": 5,
          "value": "Party (join)"
        },
        {
          "level": 6,
          "value": "Kafra Storage"
        },
        {
          "level": 7,
          "value": "Party (create)"
        },
        {
          "level": 8,
          "value": "-"
        },
        {
          "level": 9,
          "value": "Job Change"
        }
      ],
      "targetType": "Passive"
    },
    {
      "description": "Sword Mastery",
      "id": 2,
      "maxLevel": 10,
      "name": "SM_SWORD",
      "type": "Weapon",
      "bonusToSelf": [
        {
          "level": 1,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 4
          }
        },
        {
          "level": 2,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 8
          }
        },
        {
          "level": 3,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 12
          }
        },
        {
          "level": 4,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 16
          }
        },
        {
          "level": 5,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 20
          }
        },
        {
          "level": 6,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 24
          }
        },
        {
          "level": 7,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 28
          }
        },
        {
          "level": 8,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 32
          }
        },
        {
          "level": 9,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 36
          }
        },
        {
          "level": 10,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 40
          }
        }
      ],
      "targetType": "Passive"
    }
  ]
}"#;
        let res = JSONParser::parse(json,ParseOptions::default()).unwrap();
        let vec = res.json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }

    #[test]
    fn actual_test_data_start_at() {
        let json =
            r#"{
  "skills": [
    {
      "description": "Basic Skill",
      "id": 1,
      "maxLevel": 9,
      "name": "NV_BASIC",
      "basicSkillPerLevel": [{
          "level": 1,
          "value": "Trade"
        },
        {
          "level": 2,
          "value": "Emoticon"
        },
        {
          "level": 3,
          "value": "Sit"
        },
        {
          "level": 4,
          "value": "Chat Room (create)"
        },
        {
          "level": 5,
          "value": "Party (join)"
        },
        {
          "level": 6,
          "value": "Kafra Storage"
        },
        {
          "level": 7,
          "value": "Party (create)"
        },
        {
          "level": 8,
          "value": "-"
        },
        {
          "level": 9,
          "value": "Job Change"
        }
      ],
      "targetType": "Passive"
    },
    {
      "description": "Sword Mastery",
      "id": 2,
      "maxLevel": 10,
      "name": "SM_SWORD",
      "type": "Weapon",
      "bonusToSelf": [{
          "level": 1,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 4
          }
        },
        {
          "level": 2,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 8
          }
        },
        {
          "level": 3,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 12
          }
        },
        {
          "level": 4,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 16
          }
        },
        {
          "level": 5,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 20
          }
        },
        {
          "level": 6,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 24
          }
        },
        {
          "level": 7,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 28
          }
        },
        {
          "level": 8,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 32
          }
        },
        {
          "level": 9,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 36
          }
        },
        {
          "level": 10,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 40
          }
        }
      ],
      "targetType": "Passive"
    }
  ]
}"#;
        let res = JSONParser::parse(json,ParseOptions::default().start_parse_at("/skills".to_string()).parse_array(false)).unwrap();
        let vec = res.json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }

    #[test]
    fn actual_test_data_max_depth() {
        let json =
            r#"{
  "skills": [{
      "description": "Basic Skill",
      "id": 1,
      "maxLevel": 9,
      "name": "NV_BASIC",
      "basicSkillPerLevel": [{
          "level": 1,
          "value": "Trade"
        },
        {
          "level": 2,
          "value": "Emoticon"
        },
        {
          "level": 3,
          "value": "Sit"
        },
        {
          "level": 4,
          "value": "Chat Room (create)"
        },
        {
          "level": 5,
          "value": "Party (join)"
        },
        {
          "level": 6,
          "value": "Kafra Storage"
        },
        {
          "level": 7,
          "value": "Party (create)"
        },
        {
          "level": 8,
          "value": "-"
        },
        {
          "level": 9,
          "value": "Job Change"
        }
      ],
      "targetType": "Passive"
    },
    {
      "description": "Sword Mastery",
      "id": 2,
      "maxLevel": 10,
      "name": "SM_SWORD",
      "type": "Weapon",
      "bonusToSelf": [{
          "level": 1,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 4
          }
        },
        {
          "level": 2,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 8
          }
        },
        {
          "level": 3,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 12
          }
        },
        {
          "level": 4,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 16
          }
        },
        {
          "level": 5,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 20
          }
        },
        {
          "level": 6,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 24
          }
        },
        {
          "level": 7,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 28
          }
        },
        {
          "level": 8,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 32
          }
        },
        {
          "level": 9,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 36
          }
        },
        {
          "level": 10,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 40
          }
        }
      ],
      "targetType": "Passive"
    }
  ]
}"#;

        let res = JSONParser::parse(json,ParseOptions::default().max_depth(1).parse_array(true)).unwrap();
        let vec = res.json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }


    #[test]
    fn actual_test_data_max_depth_parse_array_false() {
        let json =
            r#"{
  "skills": [{
      "description": "Basic Skill",
      "id": 1,
      "maxLevel": 9,
      "name": "NV_BASIC",
      "basicSkillPerLevel": [{
          "level": 1,
          "value": "Trade"
        },
        {
          "level": 2,
          "value": "Emoticon"
        },
        {
          "level": 3,
          "value": "Sit"
        },
        {
          "level": 4,
          "value": "Chat Room (create)"
        },
        {
          "level": 5,
          "value": "Party (join)"
        },
        {
          "level": 6,
          "value": "Kafra Storage"
        },
        {
          "level": 7,
          "value": "Party (create)"
        },
        {
          "level": 8,
          "value": "-"
        },
        {
          "level": 9,
          "value": "Job Change"
        }
      ],
      "targetType": "Passive"
    },
    {
      "description": "Sword Mastery",
      "id": 2,
      "maxLevel": 10,
      "name": "SM_SWORD",
      "type": "Weapon",
      "bonusToSelf": [{
          "level": 1,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 4
          }
        },
        {
          "level": 2,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 8
          }
        },
        {
          "level": 3,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 12
          }
        },
        {
          "level": 4,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 16
          }
        },
        {
          "level": 5,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 20
          }
        },
        {
          "level": 6,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 24
          }
        },
        {
          "level": 7,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 28
          }
        },
        {
          "level": 8,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 32
          }
        },
        {
          "level": 9,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 36
          }
        },
        {
          "level": 10,
          "value": {
            "bonus": "MasteryDamageUsingWeaponType",
            "value": "1hSword",
            "value2": 40
          }
        }
      ],
      "targetType": "Passive"
    }
  ]
}"#;

        let res = JSONParser::parse(json,ParseOptions::default().max_depth(1).parse_array(false)).unwrap();
        let vec = res.json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }
}