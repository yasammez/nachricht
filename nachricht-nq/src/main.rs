mod parser;

use nachricht::*;
use std::io::{self, Read};
use anyhow::{Context, Result};
use structopt::StructOpt;
use std::str::from_utf8;
use std::path::PathBuf;
use std::fs::File;

/// Transform nachricht messages between wire format and textual representation.  By default, input is treated as binary
/// and output is generated in textual form. This behaviour can be modified by the flags below.
#[derive(StructOpt)]
#[structopt(name = "nq", author = "Liv Fischer")]
struct Opt {
    /// Encode the output into the wire format instead
    #[structopt(short, long)]
    encode: bool,

    /// Parse the input from the textual representation instead
    #[structopt(short, long)]
    text: bool,

    /// Open a nachricht encoded file in the standard editor
    #[structopt(short, long, parse(from_os_str))]
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    match opt.file {
        Some(path) => file_mode(path),
        None => streaming_mode(opt),
    }
}

fn file_mode(path: PathBuf) -> Result<()> {
    let mut buf = Vec::new();
    File::open(&path)?.read_to_end(&mut buf)?;
    let field = Decoder::decode(&buf)?.0;
    let edited = edit::edit(format!("{}", &field))?;
    let parsed = parser::parse(&edited)?;
    Encoder::encode(&parsed, &mut File::create(&path)?)?;
    Ok(())
}

fn streaming_mode(opt: Opt) -> Result<()> {
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer).context("Failed to read stdin")?;
    let field = if opt.text {
        parse(&buffer)?
    } else {
        Decoder::decode(&buffer)?.0
    };
    if opt.encode {
        Encoder::encode(&field, &mut io::stdout())?;
    } else {
        println!("{}", &field);
    }
    Ok(())
}

fn parse(buffer: &[u8]) -> Result<Field> {
    let string = from_utf8(&buffer).context("input is not utf-8")?;
    parser::parse(string)
}
