use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::time::Instant;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::serializer::{serialize_to_json, Value};

pub mod parser;
pub mod lexer;
mod serializer;

pub struct JSONParser {}

#[derive(Clone)]
pub struct ParseOptions {
    pub parse_array: bool,
    pub keep_object_raw_data: bool,
    pub max_depth: u8,
    pub start_parse_at: Option<String>,
    pub prefix: Option<String>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            parse_array: true,
            keep_object_raw_data: true,
            max_depth: 10,
            start_parse_at: None,
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

#[derive(Debug, Clone)]
pub struct JsonArrayEntries<'json> {
    pub entries: FlatJsonValue<'json>,
    pub index: usize,
}

impl<'json> JsonArrayEntries<'json> {
    pub fn entries(&self) -> &FlatJsonValue {
        &self.entries
    }
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn find_node_at(&'json self, pointer: &str) -> Option<&(PointerKey, Option<&'json str>)> {
        self.entries().iter().find(|(p, _)| p.pointer.eq(pointer))
    }
}

#[derive(Debug, Clone)]
pub struct JsonArrayEntriesOwned {
    pub entries: FlatJsonValueOwned,
    pub index: usize,
}

impl Hash for JsonArrayEntriesOwned {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.entries.len().hash(state);
    }
}

impl JsonArrayEntriesOwned {
    pub fn entries(&self) -> &FlatJsonValueOwned {
        &self.entries
    }
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn find_node_at(&self, pointer: &str) -> Option<&(PointerKey, Option<String>)> {
        self.entries().iter().find(|(p, _)| p.pointer.eq(pointer))
    }
}


#[derive(Debug, Default, Clone)]
pub struct PointerKey {
    pub pointer: String,
    pub value_type: ValueType,
    pub depth: u8,    // depth of the pointed value in the json
    pub index: usize, // index in the root json array
    pub position: usize, // position on the original json
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
        let parent_pointer = if index == 0 {
            "/"
        } else {
            &self.pointer[0..index]
        };
        parent_pointer
    }
}

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

impl PointerKey {
    pub fn from_pointer(pointer: String, value_type: ValueType, depth: u8, position: usize) -> Self {
        Self {
            pointer,
            value_type,
            depth,
            position,
            index: 0,
        }
    }
    pub fn from_pointer_and_index(pointer: String, value_type: ValueType, depth: u8, index: usize, position: usize) -> Self {
        Self {
            pointer,
            value_type,
            depth,
            index,
            position,
        }
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
#[derive(Default)]
pub enum ValueType {
    Array(usize),
    Object(bool), // parsed or not
    Number,
    String,
    Bool,
    Null,
    #[default]
    None,
}


type PointerFragment = Vec<String>;

pub type FlatJsonValue<'a> = Vec<(PointerKey, Option<&'a str>)>;
pub type FlatJsonValueOwned = Vec<(PointerKey, Option<String>)>;


#[derive(Clone)]
pub struct ParseResultRef<'json> {
    pub json: FlatJsonValue<'json>,
    pub max_json_depth: usize,
    pub parsing_max_depth: u8,
    pub started_parsing_at: Option<String>,
    pub parsing_prefix: Option<String>,
    pub depth_after_start_at: u8,
}

#[derive(Clone)]
pub struct ParseResultOwned {
    pub json: FlatJsonValueOwned,
    pub max_json_depth: usize,
    pub parsing_max_depth: u8,
    pub started_parsing_at: Option<String>,
    pub parsing_prefix: Option<String>,
    pub depth_after_start_at: u8,
}

impl<'json> ParseResultRef<'json> {
    pub fn clone_except_json(&self) -> Self {
        Self {
            json: Default::default(),
            max_json_depth: self.max_json_depth,
            parsing_max_depth: self.parsing_max_depth,
            started_parsing_at: self.started_parsing_at.clone(),
            parsing_prefix: self.parsing_prefix.clone(),
            depth_after_start_at: self.depth_after_start_at,
        }
    }

    pub fn to_owned(mut self) -> ParseResultOwned {
        let start = Instant::now();
        let mut transformed_vec: FlatJsonValueOwned = Vec::with_capacity(self.json.len());

        for (pointer, slice) in self.json {
            transformed_vec.push((pointer, slice.map(|s| s.to_owned())));
        }
        crate::ParseResultOwned {
            json: transformed_vec,
            max_json_depth: self.max_json_depth,
            parsing_max_depth: self.parsing_max_depth,
            started_parsing_at: self.started_parsing_at.clone(),
            parsing_prefix: self.parsing_prefix.clone(),
            depth_after_start_at: self.depth_after_start_at,
        }
    }
}

impl ParseResultOwned {
    pub fn clone_except_json(&self) -> Self {
        Self {
            json: Default::default(),
            max_json_depth: self.max_json_depth,
            parsing_max_depth: self.parsing_max_depth,
            started_parsing_at: self.started_parsing_at.clone(),
            parsing_prefix: self.parsing_prefix.clone(),
            depth_after_start_at: self.depth_after_start_at,
        }
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


impl JSONParser {
    pub fn parse<'json>(input: &'json str, options: ParseOptions) -> Result<ParseResultRef<'json>, String> {
        let mut lexer = Lexer::new(input.as_bytes());
        let mut parser = Parser::new(&mut lexer);
        parser.parse(&options, 1)
    }

    pub fn change_depth<'json>(previous_parse_result: &mut ParseResultRef<'json>, mut parse_options: ParseOptions) -> Result<(), String> {
        let previous_parse_depth = previous_parse_result.parsing_max_depth;
        let previous_max_json_depth = previous_parse_result.max_json_depth;
        previous_parse_result.parsing_max_depth = parse_options.max_depth;
        if previous_parse_depth < parse_options.max_depth {
            let previous_len = previous_parse_result.json.len();
            for i in 0..previous_len {
                let (k, v) = &previous_parse_result.json[i];
                let mut should_parse = false;
                let mut is_object = false;
                let mut new_depth = k.depth;
                match k.value_type {
                    ValueType::Array(_) => {
                        should_parse = true && k.depth - previous_parse_result.depth_after_start_at == previous_parse_depth;
                        new_depth = k.depth + 1 - previous_parse_result.depth_after_start_at;
                    }
                    ValueType::Object(parsed) => {
                        should_parse = !parsed && k.depth - previous_parse_result.depth_after_start_at <= previous_parse_depth;
                        is_object = true;
                        new_depth = k.depth + 1 - previous_parse_result.depth_after_start_at;
                    }
                    _ => {}
                };
                if should_parse {
                    if let Some(ref v) = v {
                        let mut lexer = Lexer::new(v.as_bytes());
                        let mut parser = Parser::new_for_change_depth(&mut lexer, previous_parse_result.depth_after_start_at, previous_max_json_depth);
                        parse_options.prefix = Some(k.pointer.clone());
                        let mut res = parser.parse(&parse_options, new_depth)?;
                        if previous_parse_result.max_json_depth < res.max_json_depth {
                            previous_parse_result.max_json_depth = res.max_json_depth;
                        }

                        if res.json.len() > 0 {
                            match &res.json[0].0.value_type {
                                ValueType::Array(size) => {
                                    previous_parse_result.json[i].0.value_type = ValueType::Array(*size);
                                    res.json.swap_remove(0);
                                }
                                _ => {}
                            }
                        }

                        if is_object {
                            previous_parse_result.json[i].0.value_type = ValueType::Object(true);
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
    pub fn change_depth_owned(previous_parse_result: &mut ParseResultOwned, mut parse_options: ParseOptions) -> Result<(), String> {
        let previous_parse_depth = previous_parse_result.parsing_max_depth;
        let previous_max_json_depth = previous_parse_result.max_json_depth;
        previous_parse_result.parsing_max_depth = parse_options.max_depth;
        if previous_parse_depth < parse_options.max_depth {
            let previous_len = previous_parse_result.json.len();
            for i in 0..previous_len {
                let (k, v) = &previous_parse_result.json[i];
                let mut should_parse = false;
                let mut is_object = false;
                let mut new_depth = k.depth;
                match k.value_type {
                    ValueType::Array(_) => {
                        should_parse = true && k.depth - previous_parse_result.depth_after_start_at == previous_parse_depth;
                        new_depth = k.depth + 1 - previous_parse_result.depth_after_start_at;
                    }
                    ValueType::Object(parsed) => {
                        should_parse = !parsed && k.depth - previous_parse_result.depth_after_start_at <= previous_parse_depth;
                        is_object = true;
                        new_depth = k.depth + 1 - previous_parse_result.depth_after_start_at;
                    }
                    _ => {}
                };
                if should_parse {
                    if let Some(ref v) = v {
                        let mut lexer = Lexer::new(v.as_bytes());
                        let mut parser = Parser::new_for_change_depth(&mut lexer, previous_parse_result.depth_after_start_at, previous_max_json_depth);
                        parse_options.prefix = Some(k.pointer.clone());
                        let mut res = parser.parse(&parse_options, new_depth)?.to_owned();
                        if previous_parse_result.max_json_depth < res.max_json_depth {
                            previous_parse_result.max_json_depth = res.max_json_depth;
                        }

                        if res.json.len() > 0 {
                            match &res.json[0].0.value_type {
                                ValueType::Array(size) => {
                                    previous_parse_result.json[i].0.value_type = ValueType::Array(*size);
                                    res.json.swap_remove(0);
                                }
                                _ => {}
                            }
                        }

                        if is_object {
                            previous_parse_result.json[i].0.value_type = ValueType::Object(true);
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

    pub fn filter_non_null_column<'a>(previous_parse_result: &Vec<JsonArrayEntries<'a>>, prefix: &str, non_null_columns: &Vec<String>) -> Vec<JsonArrayEntries<'a>> {
        let mut res: Vec<JsonArrayEntries> = Vec::with_capacity(previous_parse_result.len());
        for row in previous_parse_result {
            let mut should_add_row = true;
            for pointer in non_null_columns {
                let pointer_to_find = concat_string!(prefix, "/", row.index().to_string(), pointer);
                if let Some((_, value)) = row.find_node_at(&pointer_to_find) {
                    if value.is_none() {
                        should_add_row = false;
                        break;
                    }
                } else {
                    should_add_row = false;
                    break;
                }
            }

            if should_add_row {
                res.push(row.clone());
            }
        }
        res
    }

    pub fn serialize(mut data: FlatJsonValue) -> Value {
        serialize_to_json(data)
    }
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