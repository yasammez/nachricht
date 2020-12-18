use nachricht::*;
use std::io::{self, Read};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;
    let (field, _) = Decoder::decode(&buffer)?;
    println!("{}", &field);
    Ok(())
}
