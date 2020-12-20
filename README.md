This describes the nachricht data model and wire format, for documentation of the actual Rust crates, see [the
docs](https://docs.rs/nachricht).

# nachricht

nachricht is a self-describing binary data interchange format that aims for simplicity and small wire size. It is
heavily inspired by [msgpack](https://msgpack.org/), [CBOR](https://cbor.io/) and
[RION](http://tutorials.jenkov.com/rion/rion-encoding.html), and uses symbol tables to further reduce the message size
on wire.

## Why

I made this to learn about serialization and also because I didn't see my ideas fully reflected in any of my references.
For instance, both msgpack and CBOR allow keys to be anything, which is compatible with YAML at the most (most certainly
not JSON); on the other hand, RION permits keys to be anywhere which is fine syntactically but makes it semantically
impossible to parse. In nachricht, keys are an explicit header type but not a field type, they always need to be
followed by an actual value whose name they define. In this way, fields can be named or unnamed as they please and hence
only one container type is necessary. A JSON array can be represented by a container full of unnamed fields while a JSON
map gets translated to a container in which every field is named.

## Language support

At the moment, only Rust is supported. This might change in the future. If you would like support for a specific
language, open an issue!

## Goals

### Goals
* **Be small on wire.** We don't want to waste any bits. If you never transmit messages over flaky network links, check
  out bincode, which is much simpler to interpret but doesn't pack anything.
* **Have a low code footprint.** Do not increase code size unreasonably. Also, nobody likes exploding transitive
  dependency trees: currently nachricht has no dependencies while nachricht-serde only depends on serde. Try to keep it
  at that if possible.
* **Serialize and deserialize fast.** There is, of course, a trade-off to be made here: zero-copy formats are
  insanely fast to decode but force the serializer to pre-compute a lot of pointers. If the sending side has less CPU
  than the receiving side, this isn't optimal. Also, pointers take up space on wire (seee above).
* **Be interpretable without a schema.** This does not mean that there *cannot* be a schema. In fact, I encourage you to
  use one. However, schema evolution and discovery are much simpler when schemas are optional. Also, integration with
  serde is impossible for non self-describing formats (see above).
* **Have a human-readable representation.** Interacting with the format should be as easy as `curl | jq` for
  JSON-delivering webservices. This is why nachricht-nq exists.

### Non-Goals
* **Easy skip-parsing**: this would complicate and slow down encoders by a lot. It would also slow down decoders in
  certain circumstances when the size of the decoded type is not known (when nesting containers for instance). Also, it
  would make the use of symbol tables impossible. If your usecase involves large messages with only a couple of
  interesting fields at a time, check out flatbuffers or capnp.
* **Extensibility**: extensible standards usually create a hell of incompatible implementations just so that everyone
  can have their pet feature. Let's not go there.

## Data model

There are four small or fixed (because they do not need additional size information) and five variable length types.

| Type      | Textual representation | Description                                      |
|-----------|------------------------|--------------------------------------------------|
| Null      | null                   | also known as nil or unit                        |
| Bool      | true, false            | a simple boolean                                 |
| F32       | $123.456               | 32 bit floating point number                     |
| F64       | $$123.456              | 64 bit floating point number                     |
| Int       | 123, -123              | signed 65 bit integer                            |
| Bytes     | :base64==              | opaque array of bytes, useful for nesting        |
| String    | "hello world"          | valid UTF-8 only; length in bytes not codepoints |
| Symbol    | #red                   | Same semantics as String, for enums and atoms    |
| Key       | id=, 'with spaces'=    | the following item must be a value               |
| Container | (1, "two",)            | length in values, not bytes                      |

Containers can be arbitrarily nested. Sequences are represented as containers of anonymous values, structs as containers
of named values, i.e. ones with a key. Sequences of structs profit from references to previous keys. Maps with arbitrary
key types a represented as containers with alternating key and value entries.

## Wire format

All integers and floating point numbers, including length information is stored in network byte order, that is big
endian.

The unit of a message in nachricht is called a field. A field consists of a value and an optional key, or name. An item
is either a key or a value. As keys and symbols get en- and decoded, their values are referenced in a table. Therefore,
a key can be replaced by a reference which only contains the index into this list.

Every item begins with a header which itself consists of a lead byte and zero to eight additional bytes specifying its
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

| code | binary | mnemonic | meaning           | meaning of `payload`              |
|------|--------|----------|-------------------|-----------------------------------|
|    0 |   b000 | Bin      | Bytes             | length in bytes OR a fixed type   |
|    1 |   b001 | Pos      | positive integer  | value                             |
|    2 |   b010 | Neg      | negative integer  | abs(1 + value)                    |
|    3 |   b011 | Bag      | Container         | length in fields                  |
|    4 |   b100 | Str      | String            | length in bytes                   |
|    5 |   b101 | Sym      | Symbol            | length in bytes                   |
|    6 |   b110 | Key      | Key               | length in bytes                   |
|    7 |   b111 | Ref      | Reference         | index into symbol table           |

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

The pattern has been chosen so that the octect `0x00` equals the nachricht value `null`.

### Integer encoding

Integers are split into positive and negative because in standard two-complement representation, every negative integer
has its most significant bit set, therefore rendering packing impossible. The 1-offset is to save an additional value
byte in edge cases (-256 for instance) and because having two different representations of zero would be redundant. This
creates one superfluous case of `[0x5f 0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff]` which an encoder *should* never produce
and a decoder *must* always interpet as -18,446,744,073,709,551,615. This decision has been made to shift the inevitable
redundancy problem to a less frequently used place in the parameter space. The rather unusual i65 datatype is the
smallest type that allows encoding of either u64 or i64 values.

### The symbol table

When serializing large sequences of structs in JSON or msgpack, there is a lot of redundancy in the encoding of the
keys. To alleviate this cost, every key that gets serialized is also referenced into a symbol table, the first key
getting index zero, the second key index one, and so on. When a key is repeated, for instance when serializing another
struct of the same type, a reference can be used instead of a repetition of the key. Depending on the number of keys
(and thus the size of the symbol table) and  their values, this can save a lot of bytes on wire. Furthermore, since
there can be repeated strings in value position as well (think of enum variants or erlang atoms), a code `Symbol`
exists, which has exactly the same semantics as `String` but also introduces its value into the symbol table. Because
the distinction between keys and values is important, a decoder needs to track if an encountered value is a key or a
symbol in its symbol table for correct deserialization.

## Textual representation

In order to be easy to interact with from a developer's point of view, nachricht needs to possess a textual
representation which is free of ambiguities so that humans can read and write nachricht fields from the command line.
Note that this translation doesn't necessarily have to be bijective on a binary level: encoders *should* always use the
most space-efficient form but may choose to repeat keys instead of using the symbol table and header payloads may
allocate more bytes than strictly necessary. In the textual representation all whitespace that is not part of a quoted
string, symbol or key is regarded as insignificant. In fact the reference implementation produces spaces and newlines to
improve human readability. Parsers *must* ignore any insignificant whitespace but printers are not obliged to generate
any.

### Null and Bool

These types are simply represented with the keywords `null`, `true` and `false`.

### Floats

All numbers, floats and integers, are represented in base 10 only. F32 values are prefixed with `$` and F64 values with
`$$` to make them distinguishable from integers and each other. `.` is used as decimal separator.

### Integers

Negative integers are prefixed with `-` while positive integers have no prefix. Negative zero `-0` is illegal.

### Bytes

Bytes values are prefixed with `:` and represented in standard base 64 encoding with the trailing `=` signs where
applicable.

### String

Strings are always enclosed in double quotes `"`. This character escapes itself, so a string `"` would be represented as
`""""`.

### Symbol

Symbols are prefixed with `#`. If they contain a space or one of `$,="'()#` they are enclosed in double quotes `"`. This
character escapes itself: a symbol `red` would be represented as `#red` while `red"s` would be represented as
`#"red""s"`. Quoting is suspected to be rarely necessary by virtue of most programming languages placing restrictions on
which characters can occur in an identifier.

### Key

Keys are suffixed with `=`. If they contain a space or one of `$,="'()#` they are enclosed in single quotes `'`. This
character escapes itself: a key `version` would be represented as `version=` while `version's` would be represented as
`'version''s'=`. Quoting is suspected to be rarely necessary by virtue of most programming languages placing
restrictions on which characters can occur in an identifier.

### Container

Containers are enclosed by parentheses `()` and fields within a container are suffixed by a comma `,`. Note that a
trailing comma after the last field in a container is not optional.

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
  version = 1,
  cats = (
   (
     name = "Jessica",
     species = #PrionailurusViverrinus,
   ),
   (
     name = "Wantan",
     species = #LynxLynx,
   ),
   (
     name = "Sphinx",
     species = #FelisCatus,
   ),
   (
     name = "Chandra",
     species = #PrionailurusViverrinus,
   ),
 ),
)
```

Note that, in contrast to JSON, a single named field without an enclosing container is possible: `key = "value"` is
valid.

## Prior Art and when to use it

### Binary
* **msgpack**: when you need something like nachricht that is mature and battle-tested
* **CBOR**: when you need support for streaming or something that is an IETF standard
* **RION**: when you encode mainly CSV data
* **bincode**: when schema evolution is a non-factor and simplicity and speed reign supreme
* **flatbuffers**: when you have large messages that are read a lot more frequently than written, but only partially
* **capnp**: when you would use flatbuffers but also need a mighty, capability-based RPC framework
* **ion**: when you work at amazon
* **protobuf**: when you work at google

### Textual (not necessariliy human-readable)
* **json**: when you need dead simple interop with javascript
* **ron**: when you need something so self-descriptive that you could deduce a Rust data model from it
* **toml** when your users don't understand anything past MS-DOS INI-files
* **xml**: when you need to interop with legacy SOAP services
* **yaml**: never
