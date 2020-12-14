use anyhow::{anyhow, Result};
use clap::{App, Arg};
use std::str::FromStr;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

fn get_input_file() -> Result<PathBuf> {
    let matches = App::new("PotreeConverter v2 node counter")
        .version("1.0")
        .author("Pascal Bormann")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("The hierarchy.bin file to count the nodes in")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let file = matches
        .value_of("input")
        .expect("Argument --input was missing!");
    let path = PathBuf::from_str(file)?;
    Ok(path)
}

fn count_nodes(file: &Path) -> Result<usize> {
    let mut reader = BufReader::new(File::open(file)?);
    let mut bytes = vec![];
    reader.read_to_end(&mut bytes)?;

    if bytes.len() % 22 != 0 {
        return Err(anyhow!("File size must be a multiple of 22!"));
    }

    let num_node_entries = bytes.len() / 22;
    let num_nodes = (0..num_node_entries)
        .filter(|idx| bytes[idx * 22] != 2 || bytes[idx * 22 + 1] == 0)
        .count();
    Ok(num_nodes)
}

fn main() -> Result<()> {
    let file = get_input_file()?;
    let num_nodes = count_nodes(&file)?;
    println!("{}", num_nodes);
    Ok(())
}
