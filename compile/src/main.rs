extern crate regex;

use std::process::Command;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::io::{Result, BufReader, BufWriter};
use regex::Regex;


// TODO better implementation of this
fn copy_map<F>(from: &str, to: &str, f: F) -> Result<()>
    where F: FnOnce(String) -> String {

    let mut reader = BufReader::new(File::open(from)?);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    let mut writer = BufWriter::new(File::create(to)?);
    write!(writer, "{}", f(contents))?;
    writer.flush()
}


fn replace_fetch<'a>(re: &Regex, input: &'a str) -> String {
    re.replace_all(input, "fetch(chrome.runtime.getURL($1))").into_owned()
}


fn run() -> Result<()> {
    env::set_current_dir("..")?;

    // TODO handle exit status
    Command::new("cargo")
        .arg("web")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .status()?;

    let re = Regex::new("fetch\\( *(\"[^\"]+\") *\\)").unwrap();

    copy_map("./target/wasm32-unknown-unknown/release/saltybet.js", "./static/saltybet.js", |x| replace_fetch(&re, &x))?;
    fs::copy("./target/wasm32-unknown-unknown/release/saltybet.wasm", "./static/saltybet.wasm")?;

    copy_map("./target/wasm32-unknown-unknown/release/twitch_chat.js", "./static/twitch_chat.js", |x| replace_fetch(&re, &x))?;
    fs::copy("./target/wasm32-unknown-unknown/release/twitch_chat.wasm", "./static/twitch_chat.wasm")?;

    Ok(())
}

fn main() {
    run().unwrap();
}
