use serde::{Deserialize, Serialize};
use std::io::Write;
use anyhow::{Context, Result};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Species {
    PrionailurusViverrinus,
    LynxLynx,
    FelisCatus,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Cat<'a> {
    name: &'a str,
    species: Species,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Message<'a> {
    version: u32,
    #[serde(borrow)]
    cats: Vec<Cat<'a>>,
}

fn main() -> Result<()> {
    let msg = Message {
        version: 1,
        cats: vec![
            Cat { name: "Jessica", species: Species::PrionailurusViverrinus },
            Cat { name: "Wantan", species: Species::LynxLynx },
            Cat { name: "Sphinx", species: Species::FelisCatus },
            Cat { name: "Chandra", species: Species::PrionailurusViverrinus },
        ],
    };

    let bytes = nachricht_serde::to_bytes(&msg).context("Failed to serialize cats")?;
    std::io::stdout().write_all(&bytes).context("Failed to write bytes")?;
    Ok(())
}
