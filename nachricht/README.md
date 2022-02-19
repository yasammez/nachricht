# Rust bindings for the nachricht data interchange format

This is a pure Rust implementation of the binary
[nachricht](https://github.com/yasammez/nachricht/blob/master/README.md) data interchange format.

## Minimum supported Rust version
Since this crates makes use of the fallible collection API to pre-allocate Collections when deserializing values, the 
minimum required Rust version is `1.57.0`.

## Usage

Add this to your Cargo.toml:

```toml
[dependencies]
nachricht = "0.4.0"
```

Then you can construct, encode and decode nachricht messages:

```rust
use std::borrow::Cow;
use std::collections::BTreeMap;
use nachricht::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut buf = Vec::new();
    let nachricht = Value::Record(BTreeMap::from([(Cow::Borrowed("key"), Value::Str(Cow::Borrowed("value")))]));
    Encoder::encode(&nachricht, &mut buf)?;
    let decoded = Decoder::decode(&buf)?.0;
    assert_eq!(nachricht, decoded);
    Ok(())
}
```

