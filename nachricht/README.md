# Rust bindings for the nachricht data interchange format

This is a pure Rust implementation of the binary
[nachricht](https://github.com/yasammez/nachricht/blob/master/README.md) data interchange format.

## Usage

Add this to your Cargo.toml:

```toml
[dependencies]
nachricht = "0.1.0"
```

Then you can construct, encode and decode nachricht messages:

```rust
use std::borrow::Cow;
use nachricht::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut buf = Vec::new();
    let nachricht = Field { name: Some(Cow::Borrowed("key")), value: Value::Bool(true) };
    Encoder::encode(&nachricht, &mut buf)?;
    let decoded = Decoder::decode(&buf)?.0;
    assert_eq!(nachricht, decoded);
    Ok(())
}
```

