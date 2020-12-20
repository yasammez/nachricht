mod parser;

use nachricht::*;
use std::io::{self, Read};
use anyhow::{Context, Result, anyhow};
use structopt::StructOpt;
use std::str::from_utf8;

/// Decode and print nachricht messages
#[derive(StructOpt)]
#[structopt(name = "nq", author = "Liv Fischer")]
struct Opt {
    /// parse a textual representation and encode it iinto a binary nachricht instead
    #[structopt(short, long)]
    encode: bool,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer).context("Failed to read stdin")?;
    if opt.encode {
        parse(&buffer)
    } else {
        print(&buffer)
    }
}

fn print(buffer: &[u8]) -> Result<()> {
    let (field, _) = Decoder::decode(&buffer).context("Decoding error")?;
    println!("{}", &field);
    Ok(())
}

fn parse(buffer: &[u8]) -> Result<()> {
    let string = from_utf8(&buffer).context("input is not utf-8")?;
    let field = parser::parse(string)?;
    println!("{}", &field);
    //Encoder::encode(&field, &mut std::io::stdout())?;
    Ok(())
}
