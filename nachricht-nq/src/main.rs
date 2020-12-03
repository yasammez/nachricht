use nachricht::*;
use std::io::{self, Read};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;
    let (field, tail) = Field::decode(&buffer)?;
    println!("{}", print_field(&field));
    Ok(())
}

fn print_field(field: &Field) -> String {
    let name = match field.name {
        Some(n) => format!("@{}:", n), // TODO: escaping, richtiges Zeichen? & # ' ! ...
        None => "".into(),
    };
    let value = match &field.value {
        Value::Unit => "null".into(), // TODO: richtiges Wording?
        Value::Bool(v @ true) => "true".into(),
        Value::Bool(v @ false) => "false".into(),
        Value::F32(f) => format!("{}", f),
        Value::F64(f) => format!("{}", f),
        Value::Bytes(bytes) => format!("{:02x?}", bytes),
        Value::Int(s, num) => format!("{}{}", match s { Sign::Pos => "+", Sign::Neg => "-" }, num),
        Value::Str(value) => format!("\"{}\"", value), // TODO: escaping
        Value::Container(fields) => format!("({})", fields.iter().map(|field| print_field(field)).collect::<Vec<String>>().join(",")),
    };
    format!("{}{}", name, value)
}
