use std::{fs};
use std::path::Path;
use std::time::Instant;
use json_flat_parser::{JSONParser, ParseOptions};
// Parse
// dev - release
// 22000 - 4800: initial
// 19500 - 4500: swar for consume_string_until_end_of_array and consume_string_until_end_of_object
// 17430 - 4200: Remove skip whitespace usage
// 16275 - 4000: concat_route improvement
// 16275 - 4000: swar for text between ""

// Serialize
// 6000 : initial
// 5700 : avoid get_mut by keeping previous parent
// 4100: avoid allocate too large array
// 2300: do not create new map with default capacity
// 1700: use better sort algorithm (unstable) with better worse case performance

fn main() {
    // run: unzip skill-test.zip skill-test.json

    let path = Path::new("examples/skill-test.json");
    let mut content = fs::read_to_string(path).unwrap();
    let metadata1 = fs::metadata(path).unwrap();

    let size = metadata1.len() / 1024 / 1024;

    let start = Instant::now();

    let options = ParseOptions::default().parse_array(true).keep_object_raw_data(true).start_parse_at("/skills".to_string()).max_depth(1);
    let mut result = JSONParser::parse(content.as_mut_str(), options.clone()).unwrap();
    println!("Custom parser took {}ms for a {}mb file, max depth {}, {}", start.elapsed().as_millis(), size, result.parsing_max_depth, result.json.len());

    let start = Instant::now();
    JSONParser::change_depth(&mut result, options.clone().max_depth(2)).unwrap();
    println!("Change depth to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());
    let start = Instant::now();
    JSONParser::change_depth(&mut result, options.clone().max_depth(3)).unwrap();
    println!("Change depth to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());
    let start = Instant::now();
    JSONParser::change_depth(&mut result, options.clone().max_depth(4)).unwrap();
    println!("Change depth to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());
    let start = Instant::now();
    JSONParser::change_depth(&mut result, options.clone().max_depth(5)).unwrap();
    println!("Change depth to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());
    let start = Instant::now();
    JSONParser::change_depth(&mut result, options.clone().max_depth(6)).unwrap();
    println!("Change depth to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());

    let start = Instant::now();
    let owned = result.to_owned();
    println!("to owned took {} ms, {}", start.elapsed().as_millis(), owned.json.len());


    // let options = ParseOptions::default().parse_array(true).keep_object_raw_data(true).start_parse_at("/skills".to_string()).max_depth(1);
    // let mut result = JSONParser::parse(content.as_mut_str(), options.clone()).unwrap().to_owned();
    // println!("Custom parser took {}ms for a {}mb file, max depth {}, {}", start.elapsed().as_millis(), size, result.parsing_max_depth, result.json.len());
    // let start = Instant::now();
    // JSONParser::change_depth_owned(&mut result, options.clone().max_depth(2)).unwrap();
    // println!("Change depth owned to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());
    // let start = Instant::now();
    // JSONParser::change_depth_owned(&mut result, options.clone().max_depth(3)).unwrap();
    // println!("Change depth owned to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());
    // let start = Instant::now();
    // JSONParser::change_depth_owned(&mut result, options.clone().max_depth(4)).unwrap();
    // println!("Change depth owned to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());
    // let start = Instant::now();
    // JSONParser::change_depth_owned(&mut result, options.clone().max_depth(5)).unwrap();
    // println!("Change depth owned to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());
    // let start = Instant::now();
    // JSONParser::change_depth_owned(&mut result, options.clone().max_depth(6)).unwrap();
    // println!("Change depth owned to {} took {} ms, new json len {}", result.parsing_max_depth, start.elapsed().as_millis(), result.json.len());


    let options = ParseOptions::default().parse_array(true).keep_object_raw_data(true).start_parse_at("/skills".to_string()).max_depth(6);
    let result = JSONParser::parse(content.as_mut_str(), options.clone()).unwrap();
    println!("Custom parser took {}ms for a {}mb file, max depth {}, {}", start.elapsed().as_millis(), size, result.parsing_max_depth, result.json.len());
    let start = Instant::now();
    let owned = result.to_owned();
    println!("to owned took {} ms, {}", start.elapsed().as_millis(), owned.json.len());
    // let start = Instant::now();
    // let value = JSONParser::serialize(result.json);
    // value.to_json();
    // println!("Serialization took {}ms", start.elapsed().as_millis());
    // let mut sorted_data = result.json;
    // sorted_data.sort_by(|(a, _), (b, _)|
    //     a.pointer.cmp(&b.pointer));
    // for (pointer, v) in sorted_data.iter() {
    //     println!("{} => {:?}", pointer.pointer, v)
    // }
}