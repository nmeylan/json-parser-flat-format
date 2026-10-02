// Measure parse time and peak memory for a given max depth, the way json-editor opens a file.
// One configuration per process so peak RSS is not polluted by a previous run.
// run: cargo run --release --example bench_depth -- <file> <max_depth> [json|jsonl] [keep_object_raw_data_max_depth]
use std::fs;
use std::time::Instant;
use json_flat_parser::{JSONParser, ParseOptions};

fn peak_rss_mb() -> u64 {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    status.lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|kb| kb.parse::<u64>().ok())
        .map_or(0, |kb| kb / 1024)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = &args[1];
    let max_depth: u8 = args[2].parse().unwrap();
    let jsonl = args.get(3).is_some_and(|f| f == "jsonl");
    let raw_max_depth: u8 = args.get(4).map_or(u8::MAX, |d| d.parse().unwrap());

    let content = fs::read(path).unwrap();
    let size = content.len() / 1024 / 1024;
    let after_read = peak_rss_mb();

    let options = ParseOptions::default().parse_array(false).max_depth(max_depth).keep_object_raw_data_max_depth(raw_max_depth);
    let start = Instant::now();
    let result = if jsonl {
        JSONParser::parse_jsonl(&content, options).unwrap()
    } else {
        JSONParser::parse_bytes_owned(&content, options).unwrap()
    };
    let raw_bytes: usize = result.json.iter()
        .filter(|e| matches!(e.pointer.value_type, json_flat_parser::ValueType::Object(_, _)))
        .filter_map(|e| e.value.as_ref().map(|v| v.len()))
        .sum();
    let pointer_bytes: usize = result.json.iter().map(|e| e.pointer.pointer.len()).sum();
    println!(
        "{} {}mb max_depth={} raw_max_depth={}: total {}ms, {} entries, max json depth {}, object raw text {}mb, pointer text {}mb, peak rss {}mb (file alone {}mb)",
        if jsonl { "jsonl" } else { "json" }, size, max_depth, raw_max_depth, start.elapsed().as_millis(), result.json.len(),
        result.max_json_depth, raw_bytes / 1024 / 1024, pointer_bytes / 1024 / 1024, peak_rss_mb(), after_read
    );
}
