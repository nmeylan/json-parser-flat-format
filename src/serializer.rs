use std::cmp::Ordering;
use std::str::FromStr;
use crate::{FlatJsonValue, ValueType};

#[cfg(feature = "indexmap")]
type Map<K, V> = indexmap::IndexMap<K, V>;
#[cfg(not(feature = "indexmap"))]
type Map<K, V> = std::collections::HashMap<K, V>;

#[derive(Debug)]
pub enum Value {
    Object(Map<String, Value>),
    ObjectSerialized(String),
    Array(Vec<Value>),
    ArraySerialized(String),
    Number(f64),
    String(String),
    Bool(bool),
    Null,
}

pub fn serialize_to_json(mut data: FlatJsonValue) -> Value {
    let mut root = Value::Object(new_map());
    let mut root_array = Value::Array(Vec::with_capacity(128));

    let mut root_is_obj = true;

    let mut sorted_data = data;
    sorted_data.sort_by(|(a, _), (b, _)|
        // deepest values will go first, because we will iterate in reverse order from the array to pop value
        match b.depth.cmp(&a.depth) {
            Ordering::Equal => b.position.cmp(&a.position),
            cmp => cmp,
        });

    let mut current_parent = &mut root;
    for i in 0..sorted_data.len() {
        let (key, value) = sorted_data.pop().unwrap();

        if key.pointer.is_empty() && matches!(key.value_type, ValueType::Array(_)) {
            root_is_obj = false;
            current_parent = &mut root_array;
            continue;
        }

        if key.depth == 1 {
            match current_parent {
                Value::Object(obj) => {
                    if matches!(key.value_type, ValueType::Object) {
                        obj.insert(key.pointer[0].to_string(), Value::Object(new_map()));
                    } else if matches!(key.value_type, ValueType::Array(_)) {
                        if let Some(value) = value {
                            obj.insert(key.pointer[0].to_string(), Value::ArraySerialized(value));
                        } else {
                            obj.insert(key.pointer[0].to_string(), Value::Array(Vec::with_capacity(128)));
                        }
                    } else {
                        obj.insert(key.pointer[0].to_string(), value_to_json(value, &key.value_type));
                    }
                },
                Value::Array(array) => {
                    if matches!(key.value_type, ValueType::Object) {
                        array.push(Value::Object(new_map()));
                    } else if matches!(key.value_type, ValueType::Array(_)) {
                        if let Some(value) = value {
                            array.push(Value::ArraySerialized(value));
                        } else {
                            array.push(Value::Array(Vec::with_capacity(128)));
                        }
                    } else {
                        array.push(value_to_json(value, &key.value_type));
                    }
                },
                _ => panic!("only Object is accepted for root node")
            }
        } else {
            let segments: &Vec<String> = &key.pointer;
            let mut k = "";
            let b = key.pointer[0].as_bytes()[0];
            if b >= 0x30 && b <= 0x39 {
                current_parent = &mut root_array;
            } else {
                current_parent = &mut root;
            }
            for j in 0..(segments.len() - 1) {
                let s = &segments[j];
                match current_parent {
                    Value::Object(ref mut obj) => {
                        k = s;
                        current_parent = obj.get_mut(s).expect(format!("Expected to find parent for {}, current segment {}", key.as_string(), s).as_str());
                    }
                    Value::Array(ref mut array) => {
                        k = s;
                        current_parent = array.get_mut(usize::from_str(k).unwrap()).expect(format!("Expected to find parent at index for {}, current segment {}", key.as_string(), s).as_str());
                    }
                    _ => panic!("only Object is accepted for root node")
                }
            }
            k = &segments[segments.len() - 1];
            match current_parent {
                Value::Object(obj) => {
                    if matches!(key.value_type, ValueType::Object) {
                        obj.insert(k.to_string(), Value::Object(new_map()));
                    } else if matches!(key.value_type, ValueType::Array(_)) {
                        if let Some(value) = value {
                            obj.insert(k.to_string(), Value::ArraySerialized(value));
                        } else {
                            obj.insert(k.to_string(), Value::Array(Vec::with_capacity(128)));
                        }
                    } else {
                        obj.insert(k.to_string(), value_to_json(value, &key.value_type));
                    }
                }
                Value::Array(array) => {
                    if matches!(key.value_type, ValueType::Object) {
                        array.push(Value::Object(new_map()));
                    } else if matches!(key.value_type, ValueType::Array(_)) {
                        if let Some(value) = value {
                            array.push(Value::ArraySerialized(value));
                        } else {
                            array.push(Value::Array(Vec::with_capacity(128)));
                        }
                    } else {
                        array.push(value_to_json(value, &key.value_type));
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
fn new_map() -> Map<String, Value> {
    #[cfg(feature = "indexmap")]{
        indexmap::IndexMap::with_capacity(128)
    }
    #[cfg(not(feature = "indexmap"))]{
        std::collections::HashMap::with_capacity(128)
    }
}

// Helper function to convert string values to JSON values based on ValueType
fn value_to_json(value: Option<String>, value_type: &ValueType) -> Value {
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

impl Value {
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

        let mut parser = JSONParser::new(json);
        let vec = parser.parse(ParseOptions::default()).unwrap().json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }

    #[test]
    fn simple_array() {
        let json =
            r#"[1, 2, 3]"#;

        let mut parser = JSONParser::new(json);
        let res = parser.parse(ParseOptions::default()).unwrap();
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

        let mut parser = JSONParser::new(json);
        let res = parser.parse(ParseOptions::default()).unwrap();
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

        let mut parser = JSONParser::new(json);
        let res = parser.parse(ParseOptions::default()).unwrap();
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

        let mut parser = JSONParser::new(json);
        let res = parser.parse(ParseOptions::default()).unwrap();
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

        let mut parser = JSONParser::new(json);
        let res = parser.parse(ParseOptions::default().start_parse_at(vec!["skills".to_string()]).parse_array(false)).unwrap();
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

        let mut parser = JSONParser::new(json);
        let res = parser.parse(ParseOptions::default().max_depth(1).parse_array(true)).unwrap();
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

        let mut parser = JSONParser::new(json);
        let res = parser.parse(ParseOptions::default().max_depth(1).parse_array(false)).unwrap();
        let vec = res.json;
        let value = serialize_to_json(vec);
        assert_eq!(value.to_json(), json);
    }
}