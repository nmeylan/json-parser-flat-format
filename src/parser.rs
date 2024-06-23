use std::fmt::Debug;
use std::mem;
use crate::{concat_string, FlatJsonValue, ParseOptions, ParseResult, PointerFragment, PointerKey, string_from_bytes, ValueType};
use crate::lexer::{Lexer, Token};

pub struct Parser<'a, 'json> {
    lexer: &'a mut Lexer<'json>,
    current_token: Option<Token<'json>>,
    pub state_seen_start_parse_at: bool,
    pub max_depth: usize,
    pub depth_after_start_at: u8,
}


impl<'a, 'json: 'a> Parser<'a, 'json> {
    pub fn new(lexer: &'a mut Lexer<'json>) -> Self {
        Self { lexer, current_token: None, state_seen_start_parse_at: false, max_depth: 0, depth_after_start_at: 0 }
    }
    pub fn new_for_change_depth(lexer: &'a mut Lexer<'json>, depth_after_start_at: u8, max_depth: usize) -> Self {
        Self { lexer, current_token: None, state_seen_start_parse_at: true, max_depth, depth_after_start_at }
    }

    pub fn parse(&mut self, parse_option: &ParseOptions, depth: u8) -> Result<ParseResult<&'json str>, String> {
        let mut values: Vec<FlatJsonValue<&'json str>> = Vec::with_capacity(64);
        self.next_token();
        let mut position = 0_usize;
        if let Some(current_token) = self.current_token.as_ref() {
            if matches!(current_token, Token::CurlyOpen) {
                let mut pointer_fragment: Vec<String> = Vec::with_capacity(16);
                if let Some(ref p) = parse_option.prefix { pointer_fragment.push(p.clone()) }
                let i = 0;
                // values.push((PointerKey::from_pointer("".to_string(), ValueType::Object, depth, position), None));
                self.process_object(&mut pointer_fragment, &mut values, depth, i, parse_option, &mut position)?;
                return Ok(ParseResult {
                    json: values,
                    max_json_depth: self.max_depth,
                    parsing_max_depth: parse_option.max_depth,
                    started_parsing_at: parse_option.start_parse_at.clone(),
                    parsing_prefix: parse_option.prefix.clone(),
                    depth_after_start_at: self.depth_after_start_at,
                });
            }
            if matches!(current_token, Token::SquareOpen) {
                let mut pointer_fragment: Vec<String> = Vec::with_capacity(128);
                if let Some(ref p) = parse_option.prefix { pointer_fragment.push(p.clone()) }
                let i = 0;
                let mut pointer_index = -1 as isize;
                pointer_index = values.len() as isize;

                values.push(FlatJsonValue { pointer: PointerKey::from_pointer("".to_string(), ValueType::Array(0), depth, i), value: None });
                self.process_array(&mut pointer_fragment, &mut values, depth, i + 1, parse_option, &mut position, pointer_index);
                return Ok(ParseResult {
                    json: values,
                    max_json_depth: self.max_depth,
                    parsing_max_depth: parse_option.max_depth,
                    started_parsing_at: parse_option.start_parse_at.clone(),
                    parsing_prefix: parse_option.prefix.clone(),
                    depth_after_start_at: self.depth_after_start_at,
                });
            }
            Err(format!("Expected json to start with {{ or [ but started with {:?}", current_token))
        } else {
            Err("Json is empty".to_string())
        }
    }

    fn process_object(&mut self, route: &mut PointerFragment, target: &mut Vec<FlatJsonValue<&'json str>>, depth: u8, count: usize, parse_option: &ParseOptions, position: &mut usize) -> Result<(), String> {
        if self.max_depth < depth as usize {
            self.max_depth = depth as usize;
        }
        self.next_token();
        while let Some(ref token) = self.current_token {
            match token {
                Token::String(key) => {
                    route.push(concat_string!("/", key));
                }
                _ => return Err("Expected object to have a key at this location".to_string())
            }
            self.next_token();
            if let Some(ref _token) = self.current_token {
                match self.current_token {
                    Some(ref token) if matches!(token, Token::Colon) => {
                        self.next_token();
                    }
                    _ => return Err("Expected ':' after object key".to_string()),
                }
            } else {
                return Err("Expected ':' after object key".to_string());
            }
            self.parse_value(route, target, depth, count, parse_option, position)?;
            self.next_token();


            match self.current_token {
                Some(ref token) if matches!(token, Token::Comma) => {
                    self.next_token();
                }
                Some(ref token) if matches!(token, Token::CurlyClose) => {
                    route.pop();
                    break;
                }
                Some(ref token) if matches!(token, Token::SquareClose) => {
                    // route.pop();
                    panic!("End of array should not be consumed from there, json probably wrong");
                    break;
                }
                None => break,
                _ => return Err(format!("Expected ',' or '}}' or ']' after object value, got: {:?}", self.current_token)),
            }
            route.pop();
        }
        Ok(())
    }

    fn process_array(&mut self, route: &mut PointerFragment, target: &mut Vec<FlatJsonValue<&'json str>>, depth: u8, count: usize, parse_option: &ParseOptions, position: &mut usize, pointer_index: isize) -> Result<(), String> {
        let array_start_index = self.lexer.reader_index() - 1;
        self.next_token();
        let mut i = 1;
        while let Some(ref token) = self.current_token {
            if matches!(token, Token::SquareClose) {
                if pointer_index >= 0 {
                    let PointerKey { pointer, position, depth, .. } = mem::take(&mut target[pointer_index as usize].pointer);
                    target[pointer_index as usize] = FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::Array(i), depth, position as usize), value: None };
                }
                break;
            }
            if self.should_parse_array(&route, parse_option) {
                if !self.state_seen_start_parse_at && parse_option.start_parse_at.is_some() {
                    self.state_seen_start_parse_at = true;
                    self.depth_after_start_at = depth - 1;
                }
                if depth - self.depth_after_start_at <= parse_option.max_depth {
                    route.push("/0".to_string());
                    self.parse_value(route, target, depth, count, parse_option, position);
                    route.pop();
                    self.next_token();
                    while let Some(ref token) = self.current_token {
                        if !matches!(token, Token::Comma) {
                            break;
                        }
                        self.next_token();
                        if let Some(ref _token) = self.current_token {
                            route.push(format!("/{}", i));
                            self.parse_value(route, target, depth, count, parse_option, position);
                            route.pop();
                        } else {
                            break;
                        }
                        self.next_token();
                        i += 1;
                    }
                } else {
                    if let Some(array_str) = self.lexer.consume_string_until_end_of_array(array_start_index) {
                        if pointer_index >= 0 {
                            let PointerKey { pointer, position, depth, .. } = mem::take(&mut target[pointer_index as usize].pointer);
                            target[pointer_index as usize] = FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::Array(i), depth, position as usize), value: Some(array_str) };
                        }
                        break;
                    }
                }
            } else {
                if let Some(array_str) = self.lexer.consume_string_until_end_of_array(array_start_index) {
                    if pointer_index >= 0 {
                        let PointerKey { position, depth, .. } = target[pointer_index as usize].pointer;
                        target[pointer_index as usize] = FlatJsonValue { pointer: PointerKey::from_pointer(Self::concat_route(route), ValueType::Array(i), depth, position as usize), value: Some(array_str) };
                    }
                    break;
                }
            }
        }
        Ok(())
    }

    fn parse_value(&mut self, route: &mut PointerFragment, target: &mut Vec<FlatJsonValue<&'json str>>, depth: u8, count: usize, parse_option: &ParseOptions, position: &mut usize) -> Result<(), String> {
        match self.current_token {
            Some(ref token) => match token {
                Token::CurlyOpen => {
                    if depth - self.depth_after_start_at <= parse_option.max_depth as u8 {
                        let start = self.lexer.reader_index();
                        if let Some(object_str) = self.lexer.consume_string_until_end_of_object(true) {
                            *position += 1;
                            if parse_option.keep_object_raw_data || depth - self.depth_after_start_at == parse_option.max_depth as u8 {
                                target.push(FlatJsonValue { pointer: PointerKey::from_pointer(Self::concat_route(route), ValueType::Object(depth - self.depth_after_start_at < parse_option.max_depth), depth, *position), value: Some(object_str) });
                            } else {
                                target.push(FlatJsonValue { pointer: PointerKey::from_pointer(Self::concat_route(route), ValueType::Object(true), depth, *position), value: None });
                            }
                            self.lexer.set_reader_index(start);
                            self.process_object(route, target, depth + 1, count, parse_option, position);
                        } else {
                            panic!("We should no go there! we have not found matching closing curly {}", String::from_utf8_lossy(&self.lexer.reader().data()[start..start + 1000]))
                        }
                    } else {
                        // consuming remaining token
                        self.lexer.consume_string_until_end_of_object(false);
                        // self.process_object(route, target, depth + 1, count, parse_option, position);
                    }
                    Ok(())
                }
                Token::SquareOpen => {
                    let mut pointer_index: isize = -1;
                    if depth - self.depth_after_start_at <= parse_option.max_depth as u8 {
                        *position += 1;
                        pointer_index = target.len() as isize;
                        target.push(FlatJsonValue { pointer: PointerKey::from_pointer(Self::concat_route(route), ValueType::Array(0), depth, *position), value: None });
                    }
                    self.process_array(route, target, depth + 1, count, parse_option, position, pointer_index);
                    Ok(())
                }
                Token::String(value) => {
                    if depth - self.depth_after_start_at <= parse_option.max_depth as u8 {
                        let pointer = Self::concat_route(route);
                        if let Some(ref start_parse_at) = parse_option.start_parse_at {
                            if pointer.starts_with(start_parse_at) {
                                *position += 1;
                                target.push(FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::String, depth, *position), value: Some(value) });
                            }
                        } else {
                            *position += 1;
                            target.push(FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::String, depth, *position), value: Some(value) });
                        }
                    }

                    Ok(())
                }
                Token::Number(value) => {
                    if depth - self.depth_after_start_at <= parse_option.max_depth as u8 {
                        let pointer = Self::concat_route(route);
                        if let Some(ref start_parse_at) = parse_option.start_parse_at {
                            if pointer.starts_with(start_parse_at) {
                                *position += 1;
                                target.push(FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::Number, depth, *position), value: Some(value) });
                            }
                        } else {
                            *position += 1;
                            target.push(FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::Number, depth, *position), value: Some(value) });
                        }
                    }
                    Ok(())
                }
                Token::Boolean(value) => {
                    if depth - self.depth_after_start_at <= parse_option.max_depth as u8 {
                        let pointer = Self::concat_route(route);
                        if let Some(ref start_parse_at) = parse_option.start_parse_at {
                            if pointer.starts_with(start_parse_at) {
                                *position += 1;
                                target.push(FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::Bool, depth, *position), value: Some(value) });
                            }
                        } else {
                            *position += 1;
                            target.push(FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::Bool, depth, *position), value: Some(value) });
                        }
                    }
                    Ok(())
                }
                Token::Null => {
                    if depth <= parse_option.max_depth as u8 {
                        let pointer = Self::concat_route(route);
                        if let Some(ref start_parse_at) = parse_option.start_parse_at {
                            if pointer.starts_with(start_parse_at) {
                                *position += 1;
                                target.push(FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::Null, depth, *position), value: None });
                            }
                        } else {
                            *position += 1;
                            target.push(FlatJsonValue { pointer: PointerKey::from_pointer(pointer, ValueType::Null, depth, *position), value: None });
                        }
                    }
                    Ok(())
                }
                _ => Err(format!("Unexpected token: {:?}", token))
            },
            _ => Err("Unexpected end of input".to_string())
        }
    }

    fn should_parse_array(&mut self, route: &&mut PointerFragment, parse_option: &ParseOptions) -> bool {
        parse_option.parse_array
            || parse_option.start_parse_at.is_none() && route.is_empty()
            // When parse_array is disable, we allow to parse array if we set a pointer from where we start parsing and this pointer is an array itself, otherwise we would not parse anything
            || (parse_option.start_parse_at.is_some() && !self.state_seen_start_parse_at && parse_option.start_parse_at.as_ref().unwrap().eq(&Self::concat_route(route)))
    }
    #[inline]
    fn concat_route(route: &PointerFragment) -> String {
        let mut res = String::with_capacity(64);
        for p in route {
            res.push_str(p);
        }
        res
    }
    #[inline]
    fn next_token(&mut self) {
        self.current_token = self.lexer.next_token();
    }
}


#[cfg(test)]
mod tests {
    use std::mem;
    use crate::{JSONParser, ParseOptions, ValueType};

    #[test]
    fn object() {
        let json = r#"
        {
              "id": 1,
              "maxLevel": 99,
              "name": "NV_BAS\IC\"",
              "aaa": true
            }"#;

        let mut res = JSONParser::parse(json, ParseOptions::default()).unwrap();
        let vec = res.json;
        assert_eq!(vec[0].pointer.pointer, "/id");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[0].value, Some("1"));
        assert_eq!(vec[1].pointer.pointer, "/maxLevel");
        assert_eq!(vec[1].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].value, Some("99"));
        assert_eq!(vec[2].pointer.pointer, "/name");
        assert_eq!(vec[2].pointer.value_type, ValueType::String);
        assert_eq!(vec[2].value, Some("NV_BAS\\IC\\\""));
        assert_eq!(vec[3].pointer.pointer, "/aaa");
        assert_eq!(vec[3].pointer.value_type, ValueType::Bool);
        assert_eq!(vec[3].value, Some("true"));
    }

    #[test]
    fn max_depth_object() {
        let json = r#"{"nested": {"a1": "a","b": {"a2": "a","c": {"a3": "a"}}}"#;

        let mut result1 = JSONParser::parse(json, ParseOptions::default().max_depth(1)).unwrap();
        let vec = &result1.json;
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0].pointer.pointer, "/nested");
        assert_eq!(vec[0].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[0].value, Some("{\"a1\": \"a\",\"b\": {\"a2\": \"a\",\"c\": {\"a3\": \"a\"}}}"));
        let result2 = JSONParser::parse(json, ParseOptions::default().max_depth(2)).unwrap();

        let vec = &result2.json;
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0].pointer.pointer, "/nested");
        assert_eq!(vec[0].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[0].value, Some("{\"a1\": \"a\",\"b\": {\"a2\": \"a\",\"c\": {\"a3\": \"a\"}}}"));
        assert_eq!(vec[1].pointer.pointer, "/nested/a1");
        assert_eq!(vec[1].pointer.value_type, ValueType::String);
        assert_eq!(vec[1].value, Some("a"));
        assert_eq!(vec[2].pointer.pointer, "/nested/b");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[2].value, Some("{\"a2\": \"a\",\"c\": {\"a3\": \"a\"}}"));
        JSONParser::change_depth(&mut result1, ParseOptions::default().max_depth(2)).unwrap();
        let vec = &result1.json;
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0].pointer.pointer, "/nested");
        assert_eq!(vec[0].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[0].value, Some("{\"a1\": \"a\",\"b\": {\"a2\": \"a\",\"c\": {\"a3\": \"a\"}}}"));
        assert_eq!(vec[1].pointer.pointer, "/nested/a1");
        assert_eq!(vec[1].pointer.value_type, ValueType::String);
        assert_eq!(vec[1].value, Some("a"));
        assert_eq!(vec[2].pointer.pointer, "/nested/b");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[2].value, Some("{\"a2\": \"a\",\"c\": {\"a3\": \"a\"}}"));
    }

    #[test]
    fn max_depth_object2() {
        let json = r#"{"skills": [{"description": "Bash", "bonusToTarget": [{"level":1,"value":2}], "copyflags": {
        "plagiarism": true,"reproduce": true}, "bonusToSelf": [{"level":1,"value":2}]}, {"description": "Bash", "copyflags": {"plagiarism": true,"reproduce": true}}]"#;

        let result1 = JSONParser::parse(json, ParseOptions::default().parse_array(false).start_parse_at("/skills".to_string()).max_depth(1)).unwrap();
        let vec = &result1.json;
    }

    #[test]
    fn nested_object() {
        let json = r#"
        {
              "id": 1,
              "maxLevel": 99,
              "name": "NV_BASIC",
              "aaa": true,
              "flags": {"a": true, "b": false, "c": {"nested": "Oui"}}
            }"#;

        let json = json.replace('\n', "").replace(' ', "");
        let json = json.as_str();

        let vec = JSONParser::parse(json, ParseOptions::default()).unwrap().json;
        assert_eq!(vec[0].pointer.pointer, "/id");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[0].value, Some("1"));
        assert_eq!(vec[1].pointer.pointer, "/maxLevel");
        assert_eq!(vec[1].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].value, Some("99"));
        assert_eq!(vec[2].pointer.pointer, "/name");
        assert_eq!(vec[2].pointer.value_type, ValueType::String);
        assert_eq!(vec[2].value, Some("NV_BASIC"));
        assert_eq!(vec[3].pointer.pointer, "/aaa");
        assert_eq!(vec[3].pointer.value_type, ValueType::Bool);
        assert_eq!(vec[3].value, Some("true"));
        assert_eq!(vec[4].pointer.pointer, "/flags");
        assert_eq!(vec[4].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[5].pointer.pointer, "/flags/a");
        assert_eq!(vec[5].pointer.value_type, ValueType::Bool);
        assert_eq!(vec[5].value, Some("true"));
        assert_eq!(vec[6].pointer.pointer, "/flags/b");
        assert_eq!(vec[6].pointer.value_type, ValueType::Bool);
        assert_eq!(vec[6].value, Some("false"));
        assert_eq!(vec[7].pointer.pointer, "/flags/c");
        assert_eq!(vec[7].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[8].pointer.pointer, "/flags/c/nested");
        assert_eq!(vec[8].pointer.value_type, ValueType::String);
        assert_eq!(vec[8].value, Some("Oui"));
    }

    #[test]
    fn simple_array() {
        let json = r#"
            [1,2,3]
        "#;


        let res = JSONParser::parse(json, ParseOptions::default()).unwrap();
        let vec = res.json;
        // assert_eq!(res.root_array_len, 3);
        assert_eq!(vec[0].pointer.pointer, "");
        assert_eq!(vec[0].pointer.value_type, ValueType::Array(3));
        assert_eq!(vec[1].pointer.pointer, "/0");
        assert_eq!(vec[1].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].value, Some("1"));
        assert_eq!(vec[2].pointer.pointer, "/1");
        assert_eq!(vec[2].pointer.value_type, ValueType::Number);
        assert_eq!(vec[2].value, Some("2"));
        assert_eq!(vec[3].pointer.pointer, "/2");
        assert_eq!(vec[3].pointer.value_type, ValueType::Number);
        assert_eq!(vec[3].value, Some("3"));
    }

    #[test]
    fn simple_array_nested() {
        let json = r#"
            [[1],[2],[3]]
        "#;


        let vec = JSONParser::parse(json, ParseOptions::default()).unwrap().json;
        assert_eq!(vec[0].pointer.pointer, "");
        assert_eq!(vec[0].pointer.value_type, ValueType::Array(3));
        assert_eq!(vec[1].pointer.pointer, "/0");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[2].pointer.pointer, "/0/0");
        assert_eq!(vec[2].pointer.value_type, ValueType::Number);
        assert_eq!(vec[2].value, Some("1"));

        assert_eq!(vec[3].pointer.pointer, "/1");
        assert_eq!(vec[3].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[4].pointer.pointer, "/1/0");
        assert_eq!(vec[4].pointer.value_type, ValueType::Number);
        assert_eq!(vec[4].value, Some("2"));

        assert_eq!(vec[5].pointer.pointer, "/2");
        assert_eq!(vec[5].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[6].pointer.pointer, "/2/0");
        assert_eq!(vec[6].pointer.value_type, ValueType::Number);
        assert_eq!(vec[6].value, Some("3"));
    }

    #[test]
    fn simple_array_nested_parse_false() {
        let json = r#"
            [[1],[2],[3]]
        "#;


        let vec = JSONParser::parse(json, ParseOptions::default().parse_array(false)).unwrap().json;
        println!("{:?}", vec);
        assert_eq!(vec[0].pointer.pointer, "");
        assert_eq!(vec[0].pointer.value_type, ValueType::Array(3));
        assert_eq!(vec[1].pointer.pointer, "/0");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[2].pointer.pointer, "/1");
        assert_eq!(vec[2].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[3].pointer.pointer, "/2");
        assert_eq!(vec[3].pointer.value_type, ValueType::Array(1));
    }

    #[test]
    fn array_of_object_parse_false() {
        let json = r#"
            [{"description": "Basic Skill"}, {"description": "Bash"}]
        "#;


        let vec = JSONParser::parse(json, ParseOptions::default().parse_array(false).max_depth(1)).unwrap().json;
        println!("{:?}", vec);
        assert_eq!(vec[0].pointer.pointer, "");
        assert_eq!(vec[0].pointer.value_type, ValueType::Array(2));
        assert_eq!(vec[1].pointer.pointer, "/0");
        assert_eq!(vec[1].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[2].pointer.pointer, "/1");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(false));
    }

    #[test]
    fn array() {
        let json = r#"
            {
                "skills": [
                    {"description": "Basic Skill"},
                    {"description": "Heal"},
                    {"description": "Bash"}
                ]
            }
        "#;

        let json = json.replace('\n', "").replace(' ', "");
        let json = json.as_str();

        let vec = JSONParser::parse(json, ParseOptions::default()).unwrap().json;
        assert_eq!(vec[0].pointer.pointer, "/skills");
        assert_eq!(vec[0].pointer.value_type, ValueType::Array(3));
        assert_eq!(vec[1].pointer.pointer, "/skills/0");
        assert_eq!(vec[1].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[2].pointer.pointer, "/skills/0/description");
        assert_eq!(vec[2].pointer.parent(), "/skills/0");
        assert_eq!(vec[2].pointer.value_type, ValueType::String);
        assert_eq!(vec[2].value, Some("BasicSkill"));
        assert_eq!(vec[3].pointer.pointer, "/skills/1");
        assert_eq!(vec[3].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[4].pointer.pointer, "/skills/1/description");
        assert_eq!(vec[4].pointer.value_type, ValueType::String);
        assert_eq!(vec[4].value, Some("Heal"));
        assert_eq!(vec[5].pointer.pointer, "/skills/2");
        assert_eq!(vec[5].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[6].pointer.pointer, "/skills/2/description");
        assert_eq!(vec[6].pointer.value_type, ValueType::String);
        assert_eq!(vec[6].value, Some("Bash"));
    }

    #[test]
    fn array_with_start_parse_at() {
        let json = r#"
            {
                "skills": [
                    {"description": "Basic Skill", "inner": [2]},
                    {"description": "Heal", "inner": [3]},
                    {"description": "Bash", "inner": [1]}
                ]
            }
        "#;

        let json = json.replace('\n', "").replace(' ', "");
        let json = json.as_str();

        let vec = JSONParser::parse(json, ParseOptions::default().start_parse_at("/skills".to_string()).parse_array(false)).unwrap().json;
        assert_eq!(vec.len(), 10);
        assert_eq!(vec[0].pointer.pointer, "/skills");
        assert_eq!(vec[0].pointer.value_type, ValueType::Array(3));
        assert_eq!(vec[1].pointer.pointer, "/skills/0");
        assert_eq!(vec[1].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[2].pointer.pointer, "/skills/0/description");
        assert_eq!(vec[2].pointer.value_type, ValueType::String);
        assert_eq!(vec[3].pointer.pointer, "/skills/0/inner");
        assert_eq!(vec[3].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[5].pointer.pointer, "/skills/1/description");
        assert_eq!(vec[5].pointer.value_type, ValueType::String);
        assert_eq!(vec[6].pointer.pointer, "/skills/1/inner");
        assert_eq!(vec[6].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[8].pointer.pointer, "/skills/2/description");
        assert_eq!(vec[8].pointer.value_type, ValueType::String);
        assert_eq!(vec[9].pointer.pointer, "/skills/2/inner");
        assert_eq!(vec[9].pointer.value_type, ValueType::Array(1));
    }

    #[test]
    fn array_with_parse_option_false() {
        let json = r#"
            {
                "skills": [
                    {"description": "Basic Skill"},
                    {"description": "Heal"},
                    {"description": "Bash"}
                ]
            }
        "#;


        let vec = JSONParser::parse(json, ParseOptions::default().parse_array(false)).unwrap().json;
        assert_eq!(vec[0].pointer.pointer, "/skills");
        assert_eq!(vec[0].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[0].value.unwrap().replace('\n', "").replace(' ', ""), "[{\"description\": \"Basic Skill\"},\n                    {\"description\": \"Heal\"},\n                    {\"description\": \"Bash\"}\n                ]".replace('\n', "").replace(' ', ""));
    }

    #[test]
    fn depth() {
        let json = r#"{
  "skills": [
    {
      "requires": {"spCostPerLevel": [{"level": 1, "value": 10}, {"level": 2, "value": 20}]}
    }
  ]
}"#;

        let result_ref = JSONParser::parse(json, ParseOptions::default()
            .parse_array(true).start_parse_at("/skills".to_string()).max_depth(10)).unwrap();
        let vec = result_ref.json;
        assert_eq!(vec[0].pointer.pointer, "/skills");
        assert_eq!(vec[0].pointer.depth, 1);
        assert_eq!(vec[1].pointer.pointer, "/skills/0");
        assert_eq!(vec[1].pointer.depth, 2);
        assert_eq!(vec[2].pointer.pointer, "/skills/0/requires");
        assert_eq!(vec[2].pointer.depth, 3);
        assert_eq!(vec[3].pointer.pointer, "/skills/0/requires/spCostPerLevel");
        assert_eq!(vec[3].pointer.depth, 4);
        assert_eq!(vec[4].pointer.pointer, "/skills/0/requires/spCostPerLevel/0");
        assert_eq!(vec[4].pointer.depth, 5);
        assert_eq!(vec[5].pointer.pointer, "/skills/0/requires/spCostPerLevel/0/level");
        assert_eq!(vec[5].pointer.depth, 6);
        assert_eq!(vec[6].pointer.pointer, "/skills/0/requires/spCostPerLevel/0/value");
        assert_eq!(vec[6].pointer.depth, 6);
        assert_eq!(vec[7].pointer.pointer, "/skills/0/requires/spCostPerLevel/1");
        assert_eq!(vec[7].pointer.depth, 5);
        assert_eq!(vec[8].pointer.pointer, "/skills/0/requires/spCostPerLevel/1/level");
        assert_eq!(vec[8].pointer.depth, 6);
        assert_eq!(vec[9].pointer.pointer, "/skills/0/requires/spCostPerLevel/1/value");
        assert_eq!(vec[9].pointer.depth, 6);

        let mut result_ref = JSONParser::parse(json, ParseOptions::default()
            .parse_array(true).start_parse_at("/skills".to_string()).max_depth(1)).unwrap();
        let vec = &result_ref.json;
        assert_eq!(vec[0].pointer.pointer, "/skills");
        assert_eq!(vec[0].pointer.depth, 1);
        assert_eq!(vec[1].pointer.pointer, "/skills/0");
        assert_eq!(vec[1].pointer.depth, 2);
        JSONParser::change_depth(&mut result_ref, ParseOptions::default().parse_array(true).start_parse_at("/skills".to_string()).max_depth(2)).unwrap();
        let vec = &result_ref.json;
        assert_eq!(vec[0].pointer.pointer, "/skills");
        assert_eq!(vec[0].pointer.depth, 1);
        assert_eq!(vec[1].pointer.pointer, "/skills/0");
        assert_eq!(vec[1].pointer.depth, 2);
        assert_eq!(vec[2].pointer.pointer, "/skills/0/requires");
        assert_eq!(vec[2].pointer.depth, 3);

    }

    #[test]
    fn max_depth() {
        let json = r#"{
  "aaa": 10,
  "skills": [
    {
      "description": "Basic Skill",
      "id": 1,
      "name": "NV_BASIC"
    },
    {
      "description": "Sword Mastery",
      "id": 1,
      "name": "SM_SWORD",
      "basicSkillPerLevel": [{"level": 1,"value": "Trade"}],
      "bonusToSelf": [{"level": 1, "value": {"bonus": "MasteryDamageUsingWeaponType","value": "1hSword","value2": 4}}],
      "requires": {"spCostPerLevel": [{"level": 1, "value": 10}, {"level": 2, "value": 20}]}
    }
  ]
}"#;
        let json = json.replace('\n', "").replace(' ', "");
        let json = json.as_str();

        let result_ref = JSONParser::parse(json, ParseOptions::default().max_depth(1)).unwrap();
        let vec = result_ref.json;
        assert_eq!(vec.len(), 2);
        // assert_eq!(result_ref.max_json_depth, 4);
        assert_eq!(vec[0].pointer.pointer, "/aaa");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].pointer.pointer, "/skills");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(1));
        //
        let mut res = JSONParser::parse(json, ParseOptions::default().max_depth(1)).unwrap();
        JSONParser::change_depth(&mut res, ParseOptions::default().max_depth(2)).unwrap();
        let vec = res.json;
        assert_eq!(vec.len(), 4);
        assert_eq!(vec[0].pointer.pointer, "/aaa");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].pointer.pointer, "/skills");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(2));
        assert_eq!(vec[2].pointer.pointer, "/skills/1"); // there is a swap remove
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[3].value.is_some(), true);
        assert_eq!(vec[3].pointer.pointer, "/skills/0");
        assert_eq!(vec[3].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[3].value.is_some(), true);


        let vec = JSONParser::parse(json, ParseOptions::default().max_depth(2)).unwrap().json;
        assert_eq!(vec.len(), 4);
        assert_eq!(vec[0].pointer.pointer, "/aaa");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].pointer.pointer, "/skills");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(2));
        assert_eq!(vec[2].pointer.pointer, "/skills/0");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[3].value.is_some(), true);
        assert_eq!(vec[3].pointer.pointer, "/skills/1");
        assert_eq!(vec[3].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[3].value.is_some(), true);

        //
        let mut res = JSONParser::parse(json, ParseOptions::default().max_depth(2)).unwrap();
        JSONParser::change_depth(&mut res, ParseOptions::default().max_depth(3)).unwrap();
        let vec = res.json;
        assert_eq!(vec.len(), 13);
        assert_eq!(vec[0].pointer.pointer, "/aaa");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].pointer.pointer, "/skills");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(2));
        assert_eq!(vec[2].pointer.pointer, "/skills/0");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[2].value.is_some(), true);
        assert_eq!(vec[4].pointer.pointer, "/skills/0/description");
        assert_eq!(vec[4].pointer.value_type, ValueType::String);
        assert_eq!(vec[5].pointer.pointer, "/skills/0/id");
        assert_eq!(vec[5].pointer.value_type, ValueType::Number);
        assert_eq!(vec[6].pointer.pointer, "/skills/0/name");
        assert_eq!(vec[6].pointer.value_type, ValueType::String);
        assert_eq!(vec[3].pointer.pointer, "/skills/1");
        assert_eq!(vec[3].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[3].value.is_some(), true);
        assert_eq!(vec[7].pointer.pointer, "/skills/1/description");
        assert_eq!(vec[7].pointer.value_type, ValueType::String);
        assert_eq!(vec[8].pointer.pointer, "/skills/1/id");
        assert_eq!(vec[8].pointer.value_type, ValueType::Number);
        assert_eq!(vec[9].pointer.pointer, "/skills/1/name");
        assert_eq!(vec[9].pointer.value_type, ValueType::String);
        assert_eq!(vec[10].pointer.pointer, "/skills/1/basicSkillPerLevel");
        assert_eq!(vec[10].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[11].pointer.pointer, "/skills/1/bonusToSelf");
        assert_eq!(vec[11].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[12].pointer.pointer, "/skills/1/requires");
        assert_eq!(vec[12].pointer.value_type, ValueType::Object(false));


        let vec = JSONParser::parse(json, ParseOptions::default().max_depth(3)).unwrap().json;
        assert_eq!(vec.len(), 13);
        assert_eq!(vec[0].pointer.pointer, "/aaa");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].pointer.pointer, "/skills");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(2));
        assert_eq!(vec[2].pointer.pointer, "/skills/0");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[2].value.is_some(), true);
        assert_eq!(vec[3].pointer.pointer, "/skills/0/description");
        assert_eq!(vec[3].pointer.value_type, ValueType::String);
        assert_eq!(vec[4].pointer.pointer, "/skills/0/id");
        assert_eq!(vec[4].pointer.value_type, ValueType::Number);
        assert_eq!(vec[5].pointer.pointer, "/skills/0/name");
        assert_eq!(vec[5].pointer.value_type, ValueType::String);
        assert_eq!(vec[6].pointer.pointer, "/skills/1");
        assert_eq!(vec[6].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[6].value.is_some(), true);
        assert_eq!(vec[7].pointer.pointer, "/skills/1/description");
        assert_eq!(vec[7].pointer.value_type, ValueType::String);
        assert_eq!(vec[8].pointer.pointer, "/skills/1/id");
        assert_eq!(vec[8].pointer.value_type, ValueType::Number);
        assert_eq!(vec[9].pointer.pointer, "/skills/1/name");
        assert_eq!(vec[9].pointer.value_type, ValueType::String);
        assert_eq!(vec[10].pointer.pointer, "/skills/1/basicSkillPerLevel");
        assert_eq!(vec[10].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[11].pointer.pointer, "/skills/1/bonusToSelf");
        assert_eq!(vec[11].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[12].pointer.pointer, "/skills/1/requires");
        assert_eq!(vec[12].pointer.value_type, ValueType::Object(false));


        let vec = JSONParser::parse(json, ParseOptions::default().max_depth(1).keep_object_raw_data(false)).unwrap().json;
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0].pointer.pointer, "/aaa");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].pointer.pointer, "/skills");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(1));

        let vec = JSONParser::parse(json, ParseOptions::default().max_depth(2).keep_object_raw_data(false)).unwrap().json;
        assert_eq!(vec.len(), 4);
        assert_eq!(vec[0].pointer.pointer, "/aaa");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].pointer.pointer, "/skills");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(2));
        assert_eq!(vec[2].pointer.pointer, "/skills/0");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[3].value.is_some(), true);
        assert_eq!(vec[3].pointer.pointer, "/skills/1");
        assert_eq!(vec[3].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[3].value.is_some(), true);

        let vec = JSONParser::parse(json, ParseOptions::default().max_depth(3).keep_object_raw_data(false)).unwrap().json;
        assert_eq!(vec.len(), 13);
        assert_eq!(vec[0].pointer.pointer, "/aaa");
        assert_eq!(vec[0].pointer.value_type, ValueType::Number);
        assert_eq!(vec[1].pointer.pointer, "/skills");
        assert_eq!(vec[1].pointer.value_type, ValueType::Array(2));
        assert_eq!(vec[2].pointer.pointer, "/skills/0");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[2].value.is_some(), false);
        assert_eq!(vec[3].pointer.pointer, "/skills/0/description");
        assert_eq!(vec[3].pointer.value_type, ValueType::String);
        assert_eq!(vec[4].pointer.pointer, "/skills/0/id");
        assert_eq!(vec[4].pointer.value_type, ValueType::Number);
        assert_eq!(vec[5].pointer.pointer, "/skills/0/name");
        assert_eq!(vec[5].pointer.value_type, ValueType::String);
        assert_eq!(vec[6].pointer.pointer, "/skills/1");
        assert_eq!(vec[6].pointer.value_type, ValueType::Object(true));
        assert_eq!(vec[6].value.is_some(), false);
        assert_eq!(vec[7].pointer.pointer, "/skills/1/description");
        assert_eq!(vec[7].pointer.value_type, ValueType::String);
        assert_eq!(vec[8].pointer.pointer, "/skills/1/id");
        assert_eq!(vec[8].pointer.value_type, ValueType::Number);
        assert_eq!(vec[9].pointer.pointer, "/skills/1/name");
        assert_eq!(vec[9].pointer.value_type, ValueType::String);
        assert_eq!(vec[10].pointer.pointer, "/skills/1/basicSkillPerLevel");
        assert_eq!(vec[10].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[11].pointer.pointer, "/skills/1/bonusToSelf");
        assert_eq!(vec[11].pointer.value_type, ValueType::Array(1));
        assert_eq!(vec[12].pointer.pointer, "/skills/1/requires");
        assert_eq!(vec[12].pointer.value_type, ValueType::Object(false));


        let vec = JSONParser::parse(json, ParseOptions::default().max_depth(1).start_parse_at("/skills".to_string())).unwrap().json;
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0].pointer.pointer, "/skills");
        assert_eq!(vec[0].pointer.value_type, ValueType::Array(2));
        assert_eq!(vec[1].pointer.pointer, "/skills/0");
        assert_eq!(vec[1].pointer.value_type, ValueType::Object(false));
        assert_eq!(vec[2].pointer.pointer, "/skills/1");
        assert_eq!(vec[2].pointer.value_type, ValueType::Object(false));

        let vec = JSONParser::parse(json, ParseOptions::default().max_depth(3).start_parse_at("/skills".to_string()).parse_array(false)).unwrap().json;
        assert_eq!(vec.len(), 13);
        println!("{:?}", vec);


        let mut res = JSONParser::parse(json, ParseOptions::default().start_parse_at("/skills".to_string()).max_depth(1)).unwrap();
        JSONParser::change_depth(&mut res, ParseOptions::default().start_parse_at("/skills".to_string()).max_depth(3).parse_array(false)).unwrap();
        let vec = res.json;
        println!("{:?}", vec);
        assert_eq!(vec.len(), 13);
    }

    #[test]
    fn change_depth() {
        let json = r#"
        {"skills":[{"afterCastActDelay":2000,"description":"MagnumBreak","duration2":10000,"element":"Fire","damageType":"Single","hitCount":1,"id":7,"knockback":2,"maxLevel":10,"name":"SM_MAGNUM","targetType":"Self","type":"Weapon","splashAreaPerLevel":[{"area":2,"level":1},{"area":2,"level":2},{"area":2,"level":3},{"area":2,"level":4},{"area":2,"level":5},{"area":2,"level":6},{"area":2,"level":7},{"area":2,"level":8},{"area":2,"level":9},{"area":2,"level":10},{"area":4,"level":11}],"copyflags":{"plagiarism":true,"reproduce":true},"damageflags":{"splash":true},"flags":{"targetTrap":true},"requires":{"spcost":30,"hpcostPerLevel":[{"amount":20,"level":1},{"amount":20,"level":2},{"amount":19,"level":3},{"amount":19,"level":4},{"amount":18,"level":5},{"amount":18,"level":6},{"amount":17,"level":7},{"amount":17,"level":8},{"amount":16,"level":9},{"amount":16,"level":10}]},"aoesize":"5x5square1------+++--+++--+++------","dmgAtkPerLevel":[{"level":1,"value":1.2},{"level":2,"value":1.4},{"level":3,"value":1.6},{"level":4,"value":1.8},{"level":5,"value":2},{"level":6,"value":2.2},{"level":7,"value":2.4},{"level":8,"value":2.6},{"level":9,"value":2.8},{"level":10,"value":3}],"dmgOuterPerLevel":[{"level":1,"value":0.7},{"level":2,"value":0.9},{"level":3,"value":1.1},{"level":4,"value":1.3},{"level":5,"value":1.5},{"level":6,"value":1.7},{"level":7,"value":1.9},{"level":8,"value":2.1},{"level":9,"value":2.3},{"level":10,"value":2.5}],"dmgWaves":1,"knockbackPerLevel":[{"level":1,"value":2},{"level":2,"value":2},{"level":3,"value":2},{"level":4,"value":2},{"level":5,"value":2},{"level":6,"value":2},{"level":7,"value":2},{"level":8,"value":2},{"level":9,"value":2},{"level":10,"value":2}],"bonusToSelf":[{"level":1,"value":{"bonus":"AccuracyPercentage","value":10}},{"level":2,"value":{"bonus":"AccuracyPercentage","value":20}},{"level":3,"value":{"bonus":"AccuracyPercentage","value":30}},{"level":4,"value":{"bonus":"AccuracyPercentage","value":40}},{"level":5,"value":{"bonus":"AccuracyPercentage","value":50}},{"level":6,"value":{"bonus":"AccuracyPercentage","value":60}},{"level":7,"value":{"bonus":"AccuracyPercentage","value":70}},{"level":8,"value":{"bonus":"AccuracyPercentage","value":80}},{"level":9,"value":{"bonus":"AccuracyPercentage","value":90}},{"level":10,"value":{"bonus":"AccuracyPercentage","value":100}}]}]}
        "#;
        let mut res = JSONParser::parse(json, ParseOptions::default().start_parse_at("/skills".to_string()).parse_array(false).max_depth(1)).unwrap();
        let vec = &res.json;
        assert_eq!(vec.len(), 2);
        JSONParser::change_depth(&mut res, ParseOptions::default().start_parse_at("/skills".to_string()).max_depth(2).parse_array(false)).unwrap();
        let vec = &res.json;
        // vec.iter().for_each(|e| println!("{} {} {}", e.pointer.pointer, e.pointer.depth, e.value.is_some()));
        assert_eq!(vec.len(), 25);
        JSONParser::change_depth(&mut res, ParseOptions::default().start_parse_at("/skills".to_string()).max_depth(3).parse_array(false)).unwrap();
        let vec = &res.json;
        // vec.iter().for_each(|(k, v)| println!("{} {} {}", k.pointer, k.depth, v.is_some()));
        assert_eq!(vec.len(), 31);
    }
}