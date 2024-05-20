use std::fmt::{Display};
use std::hash::{Hash, Hasher};
use crate::lexer::Lexer;
use crate::parser::Parser;

pub mod parser;
pub mod lexer;
mod serializer;

pub struct JSONParser<'a> {
    pub parser: Parser<'a>,
}

#[derive(Clone)]
pub struct ParseOptions {
    pub parse_array: bool,
    pub max_depth: usize,
    pub start_parse_at: Option<String>,
    pub prefix: Option<String>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            parse_array: true,
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
    pub fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }
    pub fn prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }
}

#[derive(Debug, Clone)]
pub struct JsonArrayEntries {
    entries: FlatJsonValue,
    index: usize,
}

impl JsonArrayEntries {
    pub fn entries(&self) -> &FlatJsonValue {
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
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
#[derive(Default)]
pub enum ValueType {
    Array,
    Object,
    Number,
    String,
    Bool,
    Null,
    #[default]
    None,
}


type PointerFragment = Vec<String>;

pub type FlatJsonValue = Vec<(PointerKey, Option<String>)>;


#[derive(Clone)]
pub struct ParseResult {
    pub json: FlatJsonValue,
    pub max_json_depth: usize,
    pub parsing_max_depth: usize,
    pub root_value_type: ValueType,
    pub started_parsing_at: Option<String>,
    pub parsing_prefix: Option<String>,
    pub root_array_len: usize,
}

impl ParseResult {
    pub fn clone_except_json(&self) -> Self {
        Self {
            json: Default::default(),
            max_json_depth: self.max_json_depth,
            parsing_max_depth: self.parsing_max_depth,
            root_value_type: Default::default(),
            started_parsing_at: self.started_parsing_at.clone(),
            parsing_prefix: self.parsing_prefix.clone(),
            root_array_len: self.root_array_len,
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


impl<'a> JSONParser<'a> {
    pub fn new(input: &'a str) -> Self {
        let lexer = Lexer::new(input.as_bytes());
        let parser = Parser::new(lexer);

        Self { parser }
    }
    pub fn parse(&mut self, options: ParseOptions) -> Result<ParseResult, String> {
        self.parser.parse(&options, 1)
    }

    pub fn change_depth(previous_parse_result: ParseResult, mut parse_options: ParseOptions) -> Result<ParseResult, String> {
        if previous_parse_result.parsing_max_depth < parse_options.max_depth {
            let previous_len = previous_parse_result.json.len();
            let mut new_flat_json_structure = FlatJsonValue::with_capacity(previous_len + (parse_options.max_depth - previous_parse_result.parsing_max_depth) * (previous_len / 3));
            for (k, v) in previous_parse_result.json {
                if !matches!(k.value_type, ValueType::Object) {
                    new_flat_json_structure.push((k, v));
                } else {
                    if k.depth == previous_parse_result.parsing_max_depth as u8 {
                        if let Some(mut v) = v {
                            new_flat_json_structure.push((k.clone(), Some(v.clone())));
                            let lexer = Lexer::new(v.as_bytes());
                            let mut parser = Parser::new(lexer);
                            parse_options.prefix = Some(k.pointer);
                            let res = parser.parse(&parse_options, k.depth + 1)?;
                            new_flat_json_structure.extend(res.json);
                        }
                    } else {
                        new_flat_json_structure.push((k, v));
                    }

                }
            }
            Ok(ParseResult {
                json: new_flat_json_structure,
                max_json_depth: previous_parse_result.max_json_depth,
                parsing_max_depth: parse_options.max_depth,
                root_value_type: previous_parse_result.root_value_type,
                started_parsing_at: previous_parse_result.started_parsing_at,
                parsing_prefix: previous_parse_result.parsing_prefix,
                root_array_len: previous_parse_result.root_array_len,
            })
        } else if previous_parse_result.parsing_max_depth > parse_options.max_depth {
            // serialization
            todo!("");
        } else {
            Ok(previous_parse_result)
        }
    }


    pub fn filter_non_null_column(previous_parse_result: &Vec<JsonArrayEntries>, prefix: &str, non_null_columns: &Vec<String>) -> Vec<JsonArrayEntries> {
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
}