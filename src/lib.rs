use std::fmt::{Debug};
use std::hash::{Hash, Hasher};

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::serializer::{serialize_to_json, Value};

pub mod parser;
pub mod lexer;
pub mod serializer;

pub struct JSONParser {}

#[derive(Clone)]
pub struct ParseOptions {
    pub parse_array: bool,
    pub keep_object_raw_data: bool,
    pub max_depth: u8,
    pub start_parse_at: Option<String>,
    pub start_depth: u8,
    pub prefix: Option<String>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            parse_array: true,
            keep_object_raw_data: true,
            max_depth: 10,
            start_parse_at: None,
            start_depth: 1,
            prefix: None,
        }
    }
}

impl ParseOptions {
    pub fn parse_array(mut self, parse_array: bool) -> Self {
        self.parse_array = parse_array;
        self
    }

    pub fn start_parse_at(mut self, pointer: String) -> Self {
        self.start_parse_at = Some(pointer);
        self
    }
    pub fn start_depth(mut self, depth: u8) -> Self {
        self.start_depth = depth;
        self
    }
    pub fn max_depth(mut self, max_depth: u8) -> Self {
        self.max_depth = max_depth;
        self
    }
    pub fn prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }
    pub fn keep_object_raw_data(mut self, keep_object_raw_data: bool) -> Self {
        self.keep_object_raw_data = keep_object_raw_data;
        self
    }
}

pub trait GetBytes {
    fn get_bytes(&self) -> &[u8];
}

impl GetBytes for String {
    fn get_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl GetBytes for &str {
    fn get_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[derive(Debug, Clone)]
pub struct JsonArrayEntries<V: Debug + Clone + AsRef<str> + GetBytes> {
    pub entries: Vec<FlatJsonValue<V>>,
    pub index: usize,
}

impl<V: Debug + Clone + AsRef<str> + GetBytes> JsonArrayEntries<V> {
    pub fn entries(&self) -> &Vec<FlatJsonValue<V>> {
        &self.entries
    }
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn find_node_at(&self, pointer: &str) -> Option<&FlatJsonValue<V>> {
        self.entries().iter().find(|v| v.pointer.pointer.eq(pointer))
    }
}

impl<V: Debug + Clone + AsRef<str> + GetBytes> Hash for JsonArrayEntries<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.entries.len().hash(state);
    }
}


#[derive(Debug, Default, Clone)]
pub struct PointerKey {
    pub pointer: String,
    pub value_type: ValueType,
    pub depth: u8,    // depth of the pointed value in the json
    pub position: usize, // position on the original json
    pub column_id: usize, // can be used to map to external object
}

impl PartialEq<Self> for PointerKey {
    fn eq(&self, other: &Self) -> bool {
        self.pointer.eq(&other.pointer)
    }
}

impl Eq for PointerKey {}

impl Hash for PointerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pointer.hash(state);
    }
}

impl PointerKey {
    pub fn parent(&self) -> &str {
        let index = self.pointer.rfind('/').unwrap_or(0);
        
        (if index == 0 {
            "/"
        } else {
            &self.pointer[0..index]
        }) as _
    }
}
#[macro_export]
macro_rules! concat_string {
    () => { String::with_capacity(0) };
    ($($s:expr),+) => {{
        use std::ops::AddAssign;
        let mut len = 0;
        $(len.add_assign(AsRef::<str>::as_ref(&$s).len());)+
        let mut buf = String::with_capacity(len);
        $(buf.push_str($s.as_ref());)+
        buf
    }};
}
macro_rules! change_depth {
    ($($t:ty, $func:ident, $to_owned:expr),+) => {$(
    pub fn $func<'json>(previous_parse_result: &mut ParseResult<$t>, mut parse_options: ParseOptions) -> Result<(), String> {
        let previous_parse_depth = previous_parse_result.parsing_max_depth;
        let previous_max_json_depth = previous_parse_result.max_json_depth;
        previous_parse_result.parsing_max_depth = parse_options.max_depth;
        if previous_parse_depth < parse_options.max_depth {
            let previous_len = previous_parse_result.json.len();
            for i in 0..previous_len {
                let entry = &previous_parse_result.json[i];
                let mut should_parse = false;
                let mut is_object = false;
                let mut new_depth = entry.pointer.depth;
                match entry.pointer.value_type {
                    ValueType::Array(_) => {
                        should_parse = parse_options.parse_array && entry.pointer.depth - previous_parse_result.depth_after_start_at == previous_parse_depth;
                        // println!("{}({:?}) - should parse: {} ({} - {} <= {})", entry.pointer.pointer, entry.pointer.value_type, should_parse, entry.pointer.depth, previous_parse_result.depth_after_start_at, previous_parse_depth);
                        new_depth = entry.pointer.depth + 1;
                    }
                    ValueType::Object(parsed, elements_count) => {
                        should_parse = !parsed && entry.pointer.depth - previous_parse_result.depth_after_start_at <= previous_parse_depth;
                        // println!("{}({:?}) - should parse: {} (!{} && {} - {} <= {})", entry.pointer.pointer, entry.pointer.value_type, should_parse, parsed, entry.pointer.depth, previous_parse_result.depth_after_start_at, previous_parse_depth);
                        is_object = true;
                        new_depth = entry.pointer.depth + 1;
                    }
                    _ => {}
                };

                if should_parse {
                    if let Some(ref v) = entry.value {
                        let mut lexer = Lexer::new(v.as_bytes());
                        let mut parser = Parser::new_for_change_depth(&mut lexer, previous_parse_result.depth_after_start_at, previous_max_json_depth);
                        parse_options.prefix = Some(entry.pointer.pointer.clone());
                        let res = parser.parse(&parse_options, new_depth).unwrap();
                        let mut res = $to_owned(res);
                        if previous_parse_result.max_json_depth < res.max_json_depth {
                            previous_parse_result.max_json_depth = res.max_json_depth;
                        }

                        // println!("{:?}", res.json);
                        if res.json.len() > 0 {
                            match &res.json[0].pointer.value_type {
                                ValueType::Array(size) => {
                                    previous_parse_result.json[i].pointer.value_type = ValueType::Array(*size);
                                    if res.json[0].pointer.pointer.eq("") {
                                        res.json.swap_remove(0); // remove array empty pointer
                                    }
                                }
                                _ => {}
                            }
                        }

                        if is_object {
                            let root_depth = previous_parse_result.json[i].pointer.depth + 1;
                            let  elements_count = res.json.iter().filter(|e| e.pointer.depth == root_depth).count();
                            previous_parse_result.json[i].pointer.value_type = ValueType::Object(true, elements_count);
                        }

                        previous_parse_result.json.extend(res.json);
                    }
                }
            }
            Ok(())
        } else {
            Ok(())
        }
    }
    )+};
}

impl PointerKey {
    pub fn from_pointer(pointer: String, value_type: ValueType, depth: u8, position: usize) -> Self {
        Self {
            pointer,
            value_type,
            depth,
            position,
            column_id: 0,
        }
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
#[derive(Default)]
pub enum ValueType {
    Array(usize),
    Object(bool, usize), // parsed or not, number of elements
    Number,
    String,
    Bool,
    Null,
    #[default]
    None,
}


type PointerFragment = Vec<String>;


#[derive(Debug, Clone, Default)]
pub struct FlatJsonValue<V: Debug + Clone + AsRef<str> + GetBytes> {
    pub pointer: PointerKey,
    pub value: Option<V>,
}


impl<V: Debug + Clone + AsRef<str> + GetBytes>  Hash for FlatJsonValue<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pointer.hash(state);
    }
}


#[derive(Debug, Clone)]
pub struct ParseResult<V: Debug + Clone + AsRef<str> + GetBytes> {
    pub json: Vec<FlatJsonValue<V>>,
    pub max_json_depth: usize,
    pub parsing_max_depth: u8,
    pub started_parsing_at: Option<String>,
    pub started_parsing_at_index_start: usize,
    pub started_parsing_at_index_end: usize,
    pub parsing_prefix: Option<String>,
    pub depth_after_start_at: u8,
}

impl ParseResult<String> {
    pub fn clone_except_json(&self) -> Self {
        Self {
            json: Default::default(),
            max_json_depth: self.max_json_depth,
            parsing_max_depth: self.parsing_max_depth,
            started_parsing_at: self.started_parsing_at.clone(),
            started_parsing_at_index_start: self.started_parsing_at_index_start,
            started_parsing_at_index_end: self.started_parsing_at_index_end,
            parsing_prefix: self.parsing_prefix.clone(),
            depth_after_start_at: self.depth_after_start_at,
        }
    }

    pub fn to_owned(self) -> ParseResult<String> {
        self
    }

}
impl ParseResult<&str> {
    pub fn clone_except_json(&self) -> Self {
        Self {
            json: Default::default(),
            max_json_depth: self.max_json_depth,
            parsing_max_depth: self.parsing_max_depth,
            started_parsing_at_index_start: self.started_parsing_at_index_start,
            started_parsing_at_index_end: self.started_parsing_at_index_end,
            started_parsing_at: self.started_parsing_at.clone(),
            parsing_prefix: self.parsing_prefix.clone(),
            depth_after_start_at: self.depth_after_start_at,
        }
    }
    pub fn to_owned(self) -> ParseResult<String> {
        let mut transformed_vec: Vec<FlatJsonValue<String>> = Vec::with_capacity(self.json.len());

        for entry in self.json {
            transformed_vec.push(FlatJsonValue { pointer: entry.pointer, value: entry.value.map(|s| s.to_owned()) });
        }
        ParseResult::<String> {
            json: transformed_vec,
            max_json_depth: self.max_json_depth,
            parsing_max_depth: self.parsing_max_depth,
            started_parsing_at_index_start: self.started_parsing_at_index_start,
            started_parsing_at_index_end: self.started_parsing_at_index_end,
            started_parsing_at: self.started_parsing_at.clone(),
            parsing_prefix: self.parsing_prefix.clone(),
            depth_after_start_at: self.depth_after_start_at,
        }
    }

}


impl JSONParser {
    pub fn parse(input: &str, options: ParseOptions) -> Result<ParseResult<&str>, String> {
        JSONParser::parse_bytes(input.as_bytes(), options)
    }
    pub fn parse_bytes(input: &[u8], options: ParseOptions) -> Result<ParseResult<&str>, String> {
        let mut lexer = Lexer::new(input);
        let mut parser = Parser::new(&mut lexer);
        parser.parse(&options, options.start_depth)
    }


    change_depth!(&'json str, change_depth, |r: ParseResult<&'json str>| r);
    change_depth!(String, change_depth_owned, |r: ParseResult<&str>| r.to_owned());


    pub fn serialize<'a>(data: &mut Vec<FlatJsonValue<&'a str>>) -> Value<&'a str> {
        serialize_to_json(data)
    }

    pub fn serialize_owned(data: &mut Vec<FlatJsonValue<String>>) -> Value<String> {
        serialize_to_json(data)
    }

    pub fn is_jsonl(input: &[u8]) -> bool {

        // Content heuristic: look for }\n{ or }\r\n{ pattern in first 4KB
        // Skip leading whitespace
        let mut i = 0;
        while i < input.len() && input[i].is_ascii_whitespace() {
            i += 1;
        }

        // Must start with { to be JSONL (not array)
        if i >= input.len() || input[i] != b'{' {
            return false;
        }

        // Look for }\n{ or }\r\n{ pattern
        let check_len = input.len().min(4096);
        let mut in_string = false;
        let mut escaped = false;

        while i < check_len {
            let ch = input[i];

            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == b'\\' {
                    escaped = true;
                } else if ch == b'"' {
                    in_string = false;
                }
                i += 1;
                continue;
            }

            match ch {
                b'"' => in_string = true,
                b'}' => {
                    // Check for }\n{ or }\r\n{
                    if i + 2 < check_len {
                        if input[i + 1] == b'\n' && input[i + 2] == b'{' {
                            return true;
                        }
                        if i + 3 < check_len && input[i + 1] == b'\r' && input[i + 2] == b'\n' && input[i + 3] == b'{' {
                            return true;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }

        false
    }

    pub fn parse_jsonl(input: &[u8], options: ParseOptions) -> Result<ParseResult<String>, String> {
        let mut all_values: Vec<FlatJsonValue<String>> = Vec::with_capacity(1024);
        let mut row_index = 0_usize;
        let mut max_depth = 0_usize;
        let mut line_number = 0_usize;

        // Root array pointer (will update size at the end)
        all_values.push(FlatJsonValue {
            pointer: PointerKey::from_pointer(String::new(), ValueType::Array(0), 1, 0),
            value: None,
        });

        // Split by newlines and process each line
        let mut line_start = 0;
        for (i, &byte) in input.iter().enumerate() {
            if byte == b'\n' || i == input.len() - 1 {
                let line_end = if byte == b'\n' { i } else { i + 1 };
                let line = &input[line_start..line_end];
                line_number += 1;

                // Trim whitespace and skip empty lines
                let trimmed = trim_ascii_whitespace(line);
                if !trimmed.is_empty() {
                    // Parse this line as a JSON object
                    let line_options = ParseOptions {
                        parse_array: options.parse_array,
                        keep_object_raw_data: options.keep_object_raw_data,
                        max_depth: options.max_depth,
                        start_parse_at: None,
                        start_depth: 2, // Objects are at depth 2 (root array is depth 1)
                        prefix: Some(format!("/{}", row_index)),
                    };

                    match Self::parse_bytes(trimmed, line_options) {
                        Ok(line_result) => {
                            max_depth = max_depth.max(line_result.max_json_depth);

                            // Count root-level elements for this object
                            let elements_count = line_result.json.iter()
                                .filter(|e| e.pointer.depth == 2)
                                .count();

                            // Add the root object entry for this row with raw content
                            let raw_content = if options.keep_object_raw_data {
                                string_from_bytes(trimmed).map(|s| s.to_owned())
                            } else {
                                None
                            };
                            all_values.push(FlatJsonValue {
                                pointer: PointerKey::from_pointer(
                                    format!("/{}", row_index),
                                    ValueType::Object(true, elements_count),
                                    2,
                                    row_index + 1,
                                ),
                                value: raw_content,
                            });

                            // Convert &str values to String and extend with parsed fields
                            for entry in line_result.json {
                                all_values.push(FlatJsonValue {
                                    pointer: entry.pointer,
                                    value: entry.value.map(|s| s.to_owned()),
                                });
                            }
                            row_index += 1;
                        }
                        Err(e) => {
                            return Err(format!("Error parsing JSONL at line {}: {}", line_number, e));
                        }
                    }
                }

                line_start = i + 1;
            }
        }

        // Update array size in root pointer
        all_values[0].pointer.value_type = ValueType::Array(row_index);

        Ok(ParseResult {
            max_json_depth: max_depth,
            parsing_max_depth: options.max_depth,
            started_parsing_at: None,
            started_parsing_at_index_start: 0,
            started_parsing_at_index_end: all_values.len().saturating_sub(1),
            json: all_values,
            parsing_prefix: None,
            depth_after_start_at: 0,
        })
    }
}

/// Trim leading and trailing ASCII whitespace from a byte slice
#[inline]
fn trim_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}


#[inline]
pub fn string_from_bytes(bytes: &[u8]) -> Option<&str> {
    #[cfg(feature = "simdutf8")]{
        simdutf8::basic::from_utf8(bytes).ok()
    }
    #[cfg(not(feature = "simdutf8"))]{
        std::str::from_utf8(bytes).ok()
    }
}

#[cfg(test)]
mod jsonl_tests {
    use super::*;

    // Detection tests
    #[test]
    fn test_is_jsonl_json_array_not_jsonl() {
        let content = b"[{\"id\": 1}, {\"id\": 2}]";
        assert!(!JSONParser::is_jsonl(content));
        assert!(!JSONParser::is_jsonl(content));
    }

    #[test]
    fn test_is_jsonl_single_object_not_jsonl() {
        let content = b"{\"id\": 1, \"name\": \"test\"}";
        assert!(!JSONParser::is_jsonl(content));
        assert!(!JSONParser::is_jsonl(content));
    }

    #[test]
    fn test_is_jsonl_by_content() {
        let content = b"{\"id\": 1}\n{\"id\": 2}";
        assert!(JSONParser::is_jsonl(content));
        assert!(JSONParser::is_jsonl(content));
    }

    #[test]
    fn test_is_jsonl_crlf() {
        let content = b"{\"id\": 1}\r\n{\"id\": 2}";
        assert!(JSONParser::is_jsonl(content));
    }

    #[test]
    fn test_is_jsonl_string_with_brace() {
        // Should not detect }\n{ inside a string
        let content = b"{\"data\": \"}\\n{\"}";
        assert!(!JSONParser::is_jsonl(content));
    }

    // Parsing tests
    #[test]
    fn test_parse_jsonl_basic() {
        let content = b"{\"id\": 1}\n{\"id\": 2}\n{\"id\": 3}";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default()).unwrap();

        // Root array
        assert_eq!(result.json[0].pointer.pointer, "");
        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(3));

        // First row object entry with raw content
        assert_eq!(result.json[1].pointer.pointer, "/0");
        assert!(matches!(result.json[1].pointer.value_type, ValueType::Object(true, 1)));
        assert_eq!(result.json[1].value, Some("{\"id\": 1}".to_string()));

        // First row field
        assert_eq!(result.json[2].pointer.pointer, "/0/id");
        assert_eq!(result.json[2].value, Some("1".to_string()));

        // Second row object entry
        assert_eq!(result.json[3].pointer.pointer, "/1");
        assert!(matches!(result.json[3].pointer.value_type, ValueType::Object(true, 1)));
        assert_eq!(result.json[3].value, Some("{\"id\": 2}".to_string()));

        // Second row field
        assert_eq!(result.json[4].pointer.pointer, "/1/id");
        assert_eq!(result.json[4].value, Some("2".to_string()));

        // Third row object entry
        assert_eq!(result.json[5].pointer.pointer, "/2");
        assert_eq!(result.json[5].value, Some("{\"id\": 3}".to_string()));

        // Third row field
        assert_eq!(result.json[6].pointer.pointer, "/2/id");
        assert_eq!(result.json[6].value, Some("3".to_string()));
    }

    #[test]
    fn test_parse_jsonl_empty_lines() {
        let content = b"{\"id\": 1}\n\n{\"id\": 2}\n\n";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default()).unwrap();

        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(2));
    }

    #[test]
    fn test_parse_jsonl_single_line() {
        let content = b"{\"id\": 1, \"name\": \"test\"}";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default()).unwrap();

        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(1));
        // Object entry at /0
        assert_eq!(result.json[1].pointer.pointer, "/0");
        assert!(matches!(result.json[1].pointer.value_type, ValueType::Object(true, 2)));
        // Fields
        assert_eq!(result.json[2].pointer.pointer, "/0/id");
        assert_eq!(result.json[3].pointer.pointer, "/0/name");
    }

    #[test]
    fn test_parse_jsonl_nested_objects() {
        let content = b"{\"user\": {\"name\": \"Alice\"}}\n{\"user\": {\"name\": \"Bob\"}}";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default()).unwrap();

        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(2));
    }

    #[test]
    fn test_parse_jsonl_different_schemas() {
        let content = b"{\"id\": 1}\n{\"name\": \"test\"}\n{\"id\": 3, \"name\": \"both\"}";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default()).unwrap();

        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(3));
    }

    #[test]
    fn test_parse_jsonl_invalid_line() {
        let content = b"{\"id\": 1}\n{invalid json}\n{\"id\": 3}";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default());
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(3));
    }

    #[test]
    fn test_parse_jsonl_unicode() {
        let content = "{\"name\": \"日本語\"}\n{\"name\": \"émoji 🎉\"}".as_bytes();
        let result = JSONParser::parse_jsonl(content, ParseOptions::default()).unwrap();

        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(2));
        // Object entry at /0
        assert_eq!(result.json[1].pointer.pointer, "/0");
        // Field with unicode value
        assert_eq!(result.json[2].pointer.pointer, "/0/name");
        assert_eq!(result.json[2].value, Some("日本語".to_string()));
    }

    #[test]
    fn test_parse_jsonl_trailing_newline() {
        let content = b"{\"id\": 1}\n{\"id\": 2}\n";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default()).unwrap();

        // Should have 2 rows, not 3
        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(2));
    }

    #[test]
    fn test_parse_jsonl_no_trailing_newline() {
        let content = b"{\"id\": 1}\n{\"id\": 2}";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default()).unwrap();

        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(2));
    }

    #[test]
    fn test_parse_jsonl_max_depth() {
        let content = b"{\"a\": {\"b\": {\"c\": 1}}}\n{\"a\": {\"b\": {\"c\": 2}}}";
        let result = JSONParser::parse_jsonl(content, ParseOptions::default().max_depth(2)).unwrap();

        // With max_depth 2, nested objects beyond that should be kept as raw strings
        assert_eq!(result.json[0].pointer.value_type, ValueType::Array(2));
    }
}