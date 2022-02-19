This describes the nachricht data model and wire format, for documentation of the actual Rust crates, see [the
docs](https://docs.rs/nachricht).

# nachricht

nachricht is a self-describing binary data interchange format that aims for simplicity and small wire size. It is
heavily inspired by [msgpack](https://msgpack.org/), [CBOR](https://cbor.io/) and
[RION](http://tutorials.jenkov.com/rion/rion-encoding.html), and uses symbol tables to further reduce the message size
on wire.

## Why

I made this to learn about serialization and also because I didn't see my ideas fully reflected in any of my references.
I heavily dislike redundancy and bloat on the wire. Using entropy coding may not always be possible and hence a proper
serialization format should have its own way of dealing with repeating data structures. It is sad that even in the 21st
century, CSV is sometimes the most efficient way to encode something. nachricht tries to fix that by assembling a symbol
table during encoding which can be used to reference already seen layouts. This way, the commonly used "array of structs"
only needs to transmit the keys once and hence is comparable in message size with purely tabular formats.

## Language support

At the moment, only Rust is supported. This might change in the future. If you would like support for a specific
language, open an issue!

## Goals

### Goals
* **Be small on wire.** We don't want to waste any bits. If you never transmit messages over flaky network links, check
  out bincode, which is much simpler to interpret but doesn't pack anything.
* **Have a low code footprint.** Do not increase code size unreasonably. Also, nobody likes exploding transitive
  dependency trees: currently nachricht has no dependencies while nachricht-serde only depends on nachricht itself and
  serde. Try to keep it at that if possible.
* **Serialize and deserialize fast.** There is, of course, a trade-off to be made here: zero-copy formats are insanely
  fast to decode but force the serializer to pre-compute a lot of pointers. If the sending side has less CPU than the
  receiving side, this isn't optimal. Also, pointers take up space on wire (see above).
* **Be interpretable without a schema.** This does not mean that there *cannot* be a schema. In fact, I encourage you to
  use one. However, schema evolution and discovery are much simpler when schemas are optional.
* **Have a human-readable representation.** Interacting with the format should be as easy as `curl | jq` for
  JSON-delivering webservices. This is why
  [nachricht-nq](https://github.com/yasammez/nachricht/tree/master/nachricht-nq) exists.

### Non-Goals
* **Easy skip-parsing**: this would complicate and slow down encoders by a lot. It would also slow down decoders in
  certain circumstances when the size of the decoded type is not known (when nesting containers for instance). Also, it
  would make the use of symbol tables impossible. If your usecase involves large messages with only a couple of
  interesting fields at a time, check out flatbuffers or capnp.
* **Extensibility**: extensible standards usually create a hell of incompatible implementations just so that everyone
  can have their pet feature. Let's not go there.

## Data model

There are four small or fixed (because they do not need additional size information) and five variable length types.

| Type      | Textual representation | Description                                                              |
|-----------|------------------------|--------------------------------------------------------------------------|
| Null      | null                   | also known as nil or unit                                                |
| Bool      | true, false            | a simple boolean                                                         |
| F32       | $123.456               | 32 bit floating point number                                             |
| F64       | $$123.456              | 64 bit floating point number                                             |
| Int       | 123, -123              | signed 65 (!) bit integer                                                |
| Bytes     | 'base64//'             | opaque array of bytes, useful for nesting                                |
| String    | "hello world"          | valid UTF-8 only; length in bytes not codepoints                         |
| Symbol    | #red                   | Same semantics as String, for enums and atoms                            |
| Array     | \[1, "two"\]           | An ordered list of other nachricht values                                |
| Map       | { "key": "value" }     | A list of key/value pairs; the keys can be of any nachricht type         |
| Record    | ( field: "value" )     | Structured data, field names are required to be strings                  |

Containers can be arbitrarily nested. Arrays of records (or recursive records) profit from references to previously seen
records of the same type. Maps have arbitrary key types but don't benefit from the reusability.

## Wire format

All integers and floating point numbers, including length information is stored in network byte order, that is big
endian.

The unit of a message in nachricht is called a value. As records and symbols get en- and decoded, their layouts/values
are referenced in a table. Therefore, a repeated symbol can be replaced by a reference which only contains the index
into this list.

Every value begins with a header which itself consists of a lead byte and zero to eight additional bytes specifying its
length. We have 256 possible states in the first byte. We want to waste none of them and simultaneously have a simple
algorithm that is easy to implement and verify. Hence we partition the byte into two parts: a three-bit code and a
five-bit unsigned integer which we shall call `sz`.

| code  | sz        |
|-------|-----------|
| x x x | y y y y y |

`sz` defines either the `payload` of the header or the amount of following bytes containing the `payload` which is
always an unsigned 64-bit integer.

| sz (code > 0) | meaning                                |
|---------------|----------------------------------------|
|  0 - 23       | `payload` equals `sz`                  |
| 24 - 31       | `payload` in `sz` - 23 following bytes |

The code defines the major type of the item.

| code | binary | mnemonic | meaning   | meaning of `payload`    |
|------|--------|----------|-----------|-------------------------|
|    0 | b000   | BIN      | Bytes     | length in bytes         |
|    1 | b001   | INT      | integer   | cf. below               |
|    2 | b010   | STR      | String    | length in bytes         |
|    3 | b011   | SYM      | Symbol    | length in bytes         |
|    4 | b100   | ARR      | Array     | length in values        |
|    5 | b101   | REC      | Record    | length in fields        |
|    6 | b110   | MAP      | Map       | length in entries       |
|    7 | b111   | REF      | Reference | index into symbol table |

Since we have five additional values which do not need size information, one type has to sacrifice these from its `sz`
parameter space, limiting the amount of values that can be encoded without additional `payload` bytes. The `Bytes` type
has been chosen for this because I expect typical payloads of that type to exceed a length of 23 in most cases anyway.
The possible values are used as following.

| sz (code = 0) | meaning                                            |
|---------------|----------------------------------------------------|
|       0       | null                                               |
|       1       | true                                               |
|       2       | false                                              |
|       3       | F32 in following four bytes                        |
|       4       | F64 in following eight bytes                       |
|  5 - 23       | Bytes with `length` equals to `sz` - 5             |
| 24 - 31       | Bytes where `length` in `sz` - 23 following bytes  |

The pattern has been chosen so that the octet `0x00` equals the nachricht value `null`.

### Integer encoding

Integers are split into positive and negative because in standard two-complement representation, every negative integer
has its most significant bit set, therefore rendering packing impossible. To fathom this split, an additional bit from 
the lead byte is used, reducing the possible `sz` values a following.

| code  | sign | sz        |
|-------|------|-----------|
| 0 0 1 | x    | y y y y   |

A sign bit of 0 means positive and a sign bit of 1 means negative integer. Since `sz` still has to account for the
possibility of up to eight following bytes, only numbers between 0 and 7 are representable in the lead byte. Positive
integers are encoded "as is" while for negative integers, the absolute of 1 plus the actual number is stored. The
1-offset is to save an additional value byte in edge cases (-256 for instance) and because having two different
representations of zero would be redundant. This creates one superfluous case
of `[0x3f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]` which an encoder *should* never produce and a decoder *must*
always interpet as -18,446,744,073,709,551,615 (-u64::MAX). This decision has been made to shift the inevitable
redundancy problem to a less frequently used place in the parameter space. The rather unusual i65 datatype is the
smallest type that allows encoding of either u64 or i64 values.

### The symbol table

When serializing large sequences of structs in JSON or msgpack, there is a lot of redundancy in the encoding of the
keys. To alleviate this cost, every record that gets serialized is also referenced into a symbol table, the first field
key getting index zero, the second key index one, and so on. Once all keys are accounted for, the whole layout as a list
of keys is inserted into the table as well. When a record layout is repeated, for instance when serializing another
struct of the same type, a reference can be used instead of a repetition of the keys. Depending on the number of keys
(and thus the size of the symbol table) and their values, this can save a lot of bytes on wire. Furthermore, since there
can be repeated strings in value position as well (think of enum variants or Erlang atoms), a code `Symbol`
exists, which has exactly the same semantics as `String` but also introduces its value into the symbol table. Because
there is only one code for references, decoders need to track the actual type (symbol or record layout) of the values
that get inserted.

## Textual representation

In order to be easy to interact with from a developer's point of view, nachricht needs to possess a textual
representation which is free of ambiguities so that humans can read and write nachricht values from the command line.
Note that this translation doesn't necessarily have to be bijective on a binary level: encoders *should* always use the
most space-efficient form but may choose to repeat layouts instead of using the symbol table and header payloads may
allocate more bytes than strictly necessary. In the textual representation all whitespace that is not part of a quoted
string, symbol or key is regarded as insignificant. In fact the reference implementation produces spaces and newlines to
improve human readability. Parsers *must* ignore any insignificant whitespace and printers are not obliged to generate
any.

### Null and Bool

These types are simply represented with the keywords `null`, `true` and `false`.

### Floats

All numbers, floats and integers, are represented in base 10 only. F32 values are prefixed with `$` and F64 values with
`$$` to make them distinguishable from integers and each other. `.` is used as decimal separator.

### Integers

Negative integers are prefixed with `-` while positive integers have no prefix. Printers *must not* produce negative
zero `-0` but parsers *should* be able to interpret and transparently convert it to `0`.

### Bytes

Bytes values are enclosed in single quotes `'` and represented in standard base64 encoding with the trailing equals
signs `=` where applicable.

### String

Strings are always enclosed in double quotes `"`. Double quotes, newlines and backslashes are escaped as `\"`, `\n` and
`\\` respectively.

### Symbol

Symbols are prefixed with `#`. If they contain a newline, space or one of `\$,:"'()[]{}#` they are enclosed in double
quotes `"` and subject to the same escaping rules as strings. A symbol `red` would be represented as `#red`
while `red"s` would be represented as `#"red\"s"`. Quoting is suspected to be rarely necessary by virtue of most
programming languages placing restrictions on which characters can occur in an identifier.

### Array

Arrays are enclosed in `[]` and contain values separated by `,`. A trailing comma is allowed but not necessary.

### Record

Records are enclosed in `()` with fields being separated by `,`. A trailing comma is allowed but not necessary. Field
keys need to be strings which are usually not quoted. If they contain a newline, space or one of `\$,:"'()[]{}#` they
are enclosed in double quotes `"` and subject to the same escaping rules as strings. Quoting is suspected to be rarely
necessary by virtue of most programming languages placing restrictions on which characters can occur in an identifier. A
colon `:` is used as a separator between the key and the field's value.

### Map

Maps are enclosed in `{}` with entries being separated by `,`. A trailing comma is allowed but not necessary. Entry keys
and values are separated by `:`. Note that unlike records, string keys in maps act just like normal strings, hence are
always required to be quoted.

### Example

Consider the following JSON:

```json
{
  "version": 1,
  "cats": [
    {
      "name": "Jessica",
      "species": "PrionailurusViverrinus"
    },
    {
      "name": "Wantan",
      "species": "LynxLynx"
    },
    {
      "name": "Sphinx",
      "species": "FelisCatus"
    },
    {
      "name": "Chandra",
      "species": "PrionailurusViverrinus"
    }
  ]
}

```

This could roughly be translated into a nachricht textual representation of:

```nachricht
(
  version: 1,
  cats: [
   (
     name: "Jessica",
     species: #PrionailurusViverrinus,
   ),
   (
     name: "Wantan",
     species: #LynxLynx,
   ),
   (
     name: "Sphinx",
     species: #FelisCatus,
   ),
   (
     name: "Chandra",
     species: #PrionailurusViverrinus,
   ),
 ],
)
```

For an explanation of the binary format of this example, check out the
[rustdoc](https://docs.rs/nachricht-serde/0.4.0/nachricht-serde/index.html) of nachricht-serde.

## Prior Art

nachricht wasn't conceived in a vacuum. The author proudly admits having been inspired by at least the following
encoding formats. This list is probably incomplete.

### Binary
* [**msgpack**](https://msgpack.org/): like nachricht but mature and battle-tested
* [**CBOR**](https://cbor.io/): supports streaming and is an IETF standard
* [**RION**](http://tutorials.jenkov.com/rion/rion-encoding.html): heavily optimized for CSV like data (nachricht aims to do the same thing but in a completely different way)
* [**bincode**](https://github.com/servo/bincode): schema evolution is a non-factor and simplicity and speed reign supreme
* [**flatbuffers**](https://google.github.io/flatbuffers/): optimized for large messages that are read a lot more frequently than written, but only partially
* [**capnp**](https://capnproto.org/index.html): direct contrahent to flatbuffers, comes with its own nifty RPC protocol including promise pipelining
* [**ion**](http://amzn.github.io/ion-docs/): seems to be optimized for extensibility
* [**protobuf**](https://developers.google.com/protocol-buffers): venerable veteran, probably inspired at least some of the above, if not all of them

### Textual (not necessariliy human-readable)
* [**json**](https://www.json.org/json-en.html): ubiquitous but maybe for a reason
* [**ron**](https://github.com/ron-rs/ron): so self-descriptive that you could deduce a Rust data model from it. The
  textual representation of nachricht is close to, but not identitcal to ron. This project does not aim to be a binary
  version of ron.