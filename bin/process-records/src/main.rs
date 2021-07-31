use std::fs::File;
use std::io::{Read, Write};
use std::io::{Result, BufReader, BufWriter};
use algorithm::record::{deserialize_records, serialize_records};


fn read(path: &str) -> Result<String> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(contents)
}

fn write(path: &str, contents: &str) -> Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    write!(writer, "{}", contents)?;
    writer.flush()
}


fn main() -> Result<()> {
    let filename = "../../SaltyBet Records.json";

    let records = deserialize_records(&read(filename)?);

    for (index, slice) in records.chunks(75_000).enumerate() {
        write(&format!("../../static/records/SaltyBet Records {}.json", index), &serialize_records(slice))?;
    }

    std::fs::remove_file(filename)?;

    Ok(())
}
