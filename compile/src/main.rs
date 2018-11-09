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
    re.replace_all(input, "fetch(chrome.runtime.getURL($1),").into_owned()
}


fn bin(name: &str) -> Result<()> {
    let re = Regex::new("fetch\\( *(\"[^\"]+\") *,").unwrap();
    copy_map(&format!("./target/wasm32-unknown-unknown/release/{}.js", name), &format!("./static/{}.js", name), |x| replace_fetch(&re, &x))?;
    fs::copy(format!("./target/wasm32-unknown-unknown/release/{}.wasm", name), format!("./static/{}.wasm", name))?;
    Ok(())
}


fn run() -> Result<()> {
    env::set_current_dir("..")?;

    let x = Command::new("rustup")
        .arg("run")
        .arg("nightly")
        .arg("cargo")
        .arg("web")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        //.arg("--runtime")
        //.arg("library-es6")
        .status()?;

    if !x.success() {
        panic!("Command failed");
    }

    bin("background")?;
    bin("saltybet")?;
    bin("twitch_chat")?;
    bin("chart")?;
    bin("records")?;
    bin("popup")?;

    Ok(())
}

fn main() {
    run().unwrap();
}
