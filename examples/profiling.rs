use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::time::Instant;
use json_flat_parser::{JSONParser, ParseOptions};

// 22000: initial
fn main() {
    // run: unzip skill-test.zip skill-test.json

    let path = Path::new("examples/skill-test.json");
    let mut content = fs::read_to_string(path).unwrap();
    let metadata1 = fs::metadata(path).unwrap();

    let size = metadata1.len() / 1024 / 1024;

    let start = Instant::now();
    let mut parser = JSONParser::new(content.as_mut_str());
    let options = ParseOptions::default().parse_array(true).max_depth(100);
    let mut result = parser.parse(options.clone()).unwrap();
    let max_depth = result.max_json_depth;
    println!("Custom parser took {}ms for a {}mb file, max depth {}, {}", start.elapsed().as_millis(), size, max_depth, result.json.len());


    // let mut sorted_data = result.json;
    // sorted_data.sort_by(|(a, _), (b, _)|
    //     a.pointer.cmp(&b.pointer));
    // for (pointer, v) in sorted_data.iter() {
    //     println!("{} => {:?}", pointer.pointer, v)
    // }

}