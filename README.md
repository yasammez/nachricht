# nachricht

## TODO
* Readme: `value` braucht einen anderen Namen
* Dokumentieren
* Beispiel
* Doctests
* Lizenz wählen (MIT)
* nq: Escaping

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

There are four small or fixed (because they do not need additional size information) and five variable length types.

+-----------+-----------------------------+------------------------+--------------------------------------------------+
| Type      | number of possible values   | Textual representation | Description                                      |
+-----------+-----------------------------+------------------------+--------------------------------------------------|
| Null      | 1                           | null                   | also known as nil or unit                        |
| Bool      | 2                           | true, false            | a simple boolean                                 |
| F32       | 2^32                        | 123.456                | 32 bit floating point number                     |
| F64       | 2^64                        | 123.456                | 64 bit floating point number                     |
+-----------+-----------------------------+------------------------+--------------------------------------------------+
| Int       | 2^65                        | 123, -123              | signed 65 bit integer                            |
| Bytes     | $\sum\_{k=0}^{2^64}(2^3)^k$ | [01, ab, d8]           | opaque array of bytes, useful for nesting        |
| String    | ?                           | "hello world"          | valid UTF-8 only; length in bytes not codepoints |
| Symbol    | ?                           | #red                   | Same semantics as String, for enums and atoms    |
| Key       | ?                           | id=, 'with spaces'=    | the following item must be a value               |
| Container | $\infty$                    | (**value**,\*)         | length in values, not bytes                      |
+-----------|-----------------------------+------------------------+--------------------------------------------------+

Containers can be arbitrarily nested. Sequences are represented as containers of anonymous values, structs as containers
of named values, i.e. ones with a key. Sequences of structs profit from references to previous keys. Maps with arbitrary
key types a represented as containers with alternating key and value entries.

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

+------+--------+------------------------------+-----------------------------------+
| code | binary | mnemonic | meaning           | meaning of `value`                |
+------+--------+----------+-------------------+-----------------------------------+
|    0 |   b000 | Bin      | Bytes             | length in bytes OR a fixed type   |
|    1 |   b001 | Pos      | positive integer  | value                             |
|    2 |   b010 | Neg      | negative integer  | abs(1 + value)                    |
|    3 |   b011 | Bag      | Container         | length in fields                  |
|    4 |   b100 | Str      | String            | length in bytes                   |
|    5 |   b101 | Sym      | Symbol            | length in bytes                   |
|    6 |   b110 | Key      | Key               | length in bytes                   |
|    7 |   b111 | Ref      | Reference         | index into symbol table           |
+------+--------+----------+-------------------------------------------------------+

+---------------+--------------------------------------+
| sz (code > 0) | meaning                              |
+---------------+--------------------------------------+
|  0 - 23       | `value` equals `sz`                  |
| 24 - 31       | `value` in `sz` - 23 following bytes |
+---------------+--------------------------------------+

This table only holds true for seven of the eight codes. Since we have five additional values which do not need size
information, one type has to sacrifice these from its `sz` parameter space, limiting the amount of values that can be
encoded without additional length bytes. The `Bytes` type has been chosen for this because I expect typical payloads of
that type to exceed a length of 23 in most cases anyway. The possible values are used as following.

+---------------+--------------------------------------+
| sz (code = 0) | meaning                              |
+---------------+--------------------------------------+
|       0       | null                                 |
|       1       | true                                 |
|       2       | false                                |
|       3       | F32 in following four bytes          |
|       4       | F64 in following eight bytes         |
|  5 - 23       | `value` equals `sz` - 5              |
| 24 - 31       | `value` in `sz` - 23 following bytes |
+---------------+--------------------------------------+

The pattern has been chosen so that the octect `0x00` equals the nachricht value `null`.

### Integer encoding

Integers are split into positive and negative because in standard two-complement representation, every negative integer
has its most significant bit set, therefore rendering packing impossible. The 1-offset is to save an additional value
byte in edge cases (-256 for instance) and because having two different representations of zero would be redundant. This
creates one superfluous case of `[0x5f 0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff]` which an encoder must never produce and
a decoder must always interpet as -18,446,744,073,709,551,615. This decision has been made to shift the inevitable
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

## Prior Art and when to use it

### Binary
* **msgpack**: when you need something like nachricht that is mature and battle-tested
* **CBOR**: when you need support for streaming or something that is an IETF standard
* **RION**: when you encode mainly CSV data
* **bincode**: when message size and schema evolution are non-factors and simplicity and speed reign supreme
* **ion**: when you work at amazon
* **flatbuffers**: when you have very large messages that get written seldom and read often but only partially
* **capnp**: when you would use flatbuffers but also need a mighty capability-based RPC framework
* **protobuf**: never

### Textual (not necessariliy human-readable)
* **json**: when you need dead simple interop with javascript
* **ron**: when you need something so self-descriptive that you could deduce a Rust data model from it
* **toml** when your users don't understand anything past MS-DOS INI-files
* **xml**: when you need to interop with legacy SOAP services
* **yaml**: never
