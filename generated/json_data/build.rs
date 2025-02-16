use std::env;
use std::fs;
use std::path::Path;

use minify::json::minify;

use json_strip::strip_jsonc_comments;

fn helper(filename: &'static str)
{
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let json_text = fs::read_to_string(filename).expect(format!("Failed to read {}", filename).as_str());
    let json_text = strip_jsonc_comments(&json_text, true);
    let json_text = minify(&json_text);

    let out_filename = format!("{}.min.json", filename);
    let out_filename = out_filename.as_str();
    let out_path = out_dir.join(out_filename);

    fs::write(out_path, json_text)
        .expect(format!("Failed to write {}", out_filename).as_str());
}

fn main() {
    helper("skippable_cutscenes.jsonc");
    helper("skippable_cutscenes_competitive.jsonc");
    helper("skippable_cutscenes_pal.jsonc");
    helper("qol.jsonc");
}
