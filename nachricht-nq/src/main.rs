use nachricht::*;
use std::io::{self, Read};
use std::error::Error;
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer).context("Failed to read stdin")?;
    let (field, _) = Decoder::decode(&buffer).context("Decoding error")?;
    println!("{}", &field);
    Ok(())
}
