use std::env;
use std::fs;
use std::path::Path;

use minify::json::minify;

use json_strip::strip_jsonc_comments;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let json_text = fs::read_to_string("skippable_cutscenes.jsonc").expect("Failed to read skippable_cutscenes.jsonc");
    let json_text = strip_jsonc_comments(&json_text, true);
    let json_text = minify(&json_text);

    let out_path = out_dir.join("skippable_cutscenes.jsonc.min.json");

    fs::write(out_path, json_text)
        .expect("Failed to write skippable_cutscenes.jsonc.min.json");
}
