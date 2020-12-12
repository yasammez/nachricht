# nachricht

## TODO
* `value` braucht einen anderen Namen
* de sollten Value nicht referenzieren
* Don't repeat yourself in ser/de implementieren
* Slicezugriffe schöner machen (ReadIO-Trait wie in CBOR?)
* Dokumentieren
* Doctests
* Readme schreiben
* Lizenz wählen
* Fehlermeldungen verbessern
* Fehlerpfade testen
* Serialisierung von Enums verbessern: braucht man so viele Container?
* nq: Escaping
* Keine \*-Imports

This is a data serialization format and implementation heavily inspired by [msgpack](https://msgpack.org/),
[CBOR](https://cbor.io/) and [RION](http://tutorials.jenkov.com/rion/rion-encoding.html).

## Why

I made this to learn about serialization and also because I didn't see my ideas fully reflected in any of my references.
For instance, both msgpack and CBOR allow keys to be anything, which is compatible with YAML at the most (most certainly
not JSON); on the other hand, RION permits keys to be anywhere which is fine syntactically but makes it semantically
impossible to parse. In Nachricht, keys are an explicit header type but not a field type, they always need to be
followed by an actual field whose name they define. In this way, fields can be named or unnamed as they please and hence
only one container type is necessary. A JSON array can be represented by a container full of unnamed fields while a JSON
map gets translated to a container in which every field is named.

## Goals

### Goals
* Small size on wire. We don't want to waste any bits. Do anything for this goal except entropy coding. Pipe the message
  through gzip if this improves your usecase. If you never transmit messages over flaky network links, check out
  bincode, which is much simpler to interpret but doesn't pack anything.
* Fast serialization and deserialization. There is, of course, a trade-off to be made here: zero-copy formats are
  insanely fast to decode but force the serializer to pre-compute a lot of pointers. If the sending side has less CPU
  than the receiving side, this isn't optimal. Also, pointers take up space on wire (seee above).
* Integrate well with serde.
* Self-describing format, that can be successfully interpreted without a schema. This does not mean that there can't be
  a schema. In fact, I encourage you to use one. However, schema evolution and discovery are much simpler when schemas
  are optional. Also, integration with serde is impossible for non self-describing formats (see above).
* Human readable representation. Interacting with the format should be as easy as `curl | jq` for JSON-delivering
  webservices. This is why nachricht-nq exists.

### Non-Goals
* Easy skip-parsing: this would complicate and slow down encoders by a lot. It would also slow down decoders in certain
  circumstances when the size of the decoded type is not known (when nesting containers for instance). Also, it would
  make the use of symbol tables impossible. If your usecase involves large messages with only a couple of interesting
  fields at a time, check out flatbuffers or capnp.
* Extensibility: extensible standards usually create a hell of incompatible implementations just so that everyone can
  have their pet feature (looking straight at you, CBOR). Let's not go there.

## Data model

There are four small or fixed (because they do not need additional size information) and six variable length types.

+-----------+-----------------------------+------------------------+-----------------------------------------------------+
| Type      | number of possible values   | Textual representation | Description                                         |
+-----------+-----------------------------+------------------------+-----------------------------------------------------|
| Unit      | 1                           | null                   | also known as nill                                  |
| Bool      | 2                           | true, false            | a simple boolean                                    |
| F32       | 2^32                        | 123.456                | 32 bit floating point number                        |
| F64       | 2^64                        | 123.456                | 64 bit floating point number                        |
+-----------+-----------------------------+------------------------+-----------------------------------------------------+
| Intp      | 2^64                        | +123                   | positive 64 bit integer                             |
| Intn      | 2^64                        | -123                   | negative 64 bit integer, encoded as abs(1 + value)  |
| Bytes     | $\sum\_{k=0}^{2^64}(2^3)^k$ | [01, ab, d8]           | opaque array of bytes, useful for nesting           |
| String    | ?                           | "hello world"          | must be valid UTF-8; length in bytes not codepoints |
| Key       | ?                           | $id, $'with spaces'    | the following item must be a value                  |
| Ref       | 2^64                        | @0, @1                 | usually not printed, refers to a previous key       | 
| Container | $\infty$                    | ( **value**,\* )       | length in values, not bytes                         |
+-----------|-----------------------------+------------------------+-----------------------------------------------------+

Containers can be arbitrarily nested. Sequences are represented as containers of anonymous values, structs as containers
of named values, i.e. ones with a key. Sequences of structs profit from references to previous keys. Maps with arbitrary
key types a represented as containers with alternating key and value entries.

Integers are split into positive and negative because in standard two-complement representation, every negative integer
has its most significant bit set, therefore rendering packing impossible. The 1-offset is to save an additional value
byte in edge cases (-256 for instance) and because having two different representations of zero would be redundant.

## Wire format

All integers and floating point numbers, including length information is stored in network byte order, that is big
endian.

The unit of a message in nachricht is called a field. A field consists of a value and an optional key, or name. An item
is either a key or a value. As keys get en- and decoded, their values are referenced in a list. Therefore, a key can be
replaced by a reference which only contains the index into this list.

Every item begins with a header which itself consists of a lead byte and zero to eight additional bytes specifying its
value. We have 256 possible states in the first byte. We want to waste none of them (looking at you, CBOR/RION) and
simultaneously have a simple algorithm that is easy to implement and verify (looking at you, msgpack). Hence we
partition the byte into two parts: a three-bit code and a five-bit unsigned integer which we shall call `sz`.

+-------+-----------+
| code  | sz        |
+-------+-----------+
| x x x | y y y y y |
+-------+-----------+

The code defines the major type of the item while `sz` defines either its `value` or length according to the following
tables.

+------+--------+-------------------+-----------------------------------+
| code | binary | meaning           | meaning of `value`                |
+------+--------+-------------------+-----------------------------------+
|    0 |   b000 | Container (lower) | length in fields OR a fixed type  |
|    1 |   b001 | Container (upper) | length in fields                  |
|    2 |   b010 | Intp              | value                             |
|    3 |   b011 | Intn              | abs(1 + value)                    |
|    4 |   b100 | Bytes             | length in bytes                   |
|    5 |   b101 | String            | length in bytes                   |
|    6 |   b110 | Key               | length in bytes                   |
|    7 |   b111 | Ref               | index into already seen keys      |
+------+--------+-------------------+-----------------------------------+

+--------------------+--------------------------------------+
| sz (code > 1)      | meaning                              |
+--------------------+--------------------------------------+
|  0 - 23            | `value` equals `sz`                  |
| 24 - 31            | `value` in 32 - `sz` following bytes |
+--------------------+--------------------------------------+

This table only holds true for five of the six variable length types. Since we only have seven big types (integers being
separated into positive and negative ones) to encode and five additional values which do not need size information,
containers get two codes, allowing for a six-bit `sz`. This case can be easily distinguished by the lead byte beginning
with two zero bits. The possible values are used as following.

+---------------------+--------------------------------------+
| sz (code <= 1)      | meaning                              |
+---------------------+--------------------------------------+
|       0             | null                                 |
|       1             | true                                 |
|       2             | false                                |
|       3             | F32 in following four bytes          |
|       4             | F64 in following eight bytes         |
|  5 - 55             | `value` equals `sz` - 5              |
| 56 - 63             | `value` in 64 - `sz` following bytes |
+---------------------+--------------------------------------+

The pattern has been chosen so that the octect `0x00` equals the nachricht value `null`.

## Prior Art and when to use it

### Binary
* **msgpack**: when you need something like nachricht that is mature and battle-tested
* **CBOR**: when you need something that is an IETF standard
* **bincode**: when message size and schema evolution are non-factors and simplicity and speed reign supreme
* **flatbuffers**: when you have very large messages that get written seldom and read often but only partially
* **capnp**: when you would use flatbuffers but also need a mighty capability-based RPC framework
* **protobuf**: never

### Textual (not necessariliy human-readable)
* **json**: when you need dead simple interop with javascript
* **ron**: when you need something so self-descriptive that you could deduce a Rust data model from it
* **toml** when your users don't understand anything past MS-DOS INI-files
* **xml**: when you need to interop with legacy SOAP services
* **yaml**: never
