mod parser;

use nachricht::*;
use std::io::{self, Read};
use anyhow::{Context, Result};
use structopt::StructOpt;
use std::str::from_utf8;

/// Decode and print nachricht messages
#[derive(StructOpt)]
#[structopt(name = "nq", author = "Liv Fischer")]
struct Opt {
    /// encode the output into the wire format instead
    #[structopt(short, long)]
    encode: bool,

    /// parse the input from the textual representation instead
    #[structopt(short, long)]
    text: bool,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
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
