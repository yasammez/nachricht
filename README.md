# nachricht

This is a data serialization format and implementation heavily inspired by msgpack, CBOR and RION.

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
  a schema. In fact, I encourage you to use one. However, schema evolution and discovery is much simpler when schemas
  are optional. Also, integration with serde is impossible for non self-describing formats (see above).
* Human readable representation. Interacting with the format should be as easy as `curl | jq` for JSON-delivering
  webservices.

### Non-Goals

* Easy skip-parsing: this would complicate and slow down encoders by a lot. It would also slow down decoders in certain
  circumstances when the size of the decoded type is not known (when nesting containers for instance). Also, it would
  make the use of symbol tables impossible. If your usecase involves large messages with only a couple of interesting
  fields at a time, check out flatbuffers or capnp.
* Extensibility: extensible standards usually create a hell of incompatible implementations just so that everyone can
  have their pet feature (looking straight at you, CBOR). Let's not go there.

## TODO

* de sollten Value nicht referenzieren
* Don't repeat yourself in ser/de implementieren
* Slicezugriffe schöner machen (ReadIO-Trait wie in CBOR?)
* Dokumentieren
* Doctests
* Readme schreiben
* Lizenz wählen
* Fehler verbessern
* Fehlerpfade testen
* Container gibt Länge in Bytes statt Elementen an?
* Serialisierung von Enums verbessern: braucht man so viele Container?
* nq: Parsing
* nq: Escaping
* Keine \*-Imports

## Redesign

We have 256 possible values in the first byte. We want to waste none of them (looking at you, CBOR/RION) and
simultaneously have a simple algorithm that is easy to implement and verify (looking at you, msgpack).

256 Zustände

+-------------------+-----------------------------+
| Wert              | Zahl an Zuständen           |
+-------------------+-----------------------------+
| Intp(direct)      | n                           |
| Intp(bytes)       | 4 o. 8                      |
| Intn(direct)      | n                           |
| Intn(bytes)       | 4 o. 8                      |
| Str(direct)       | n                           |
| Str(bytes)        | 4 o. 8                      |
| Tag(direct)       | n                           |
| Tag(bytes)        | 4 o. 8                      |
| Bytes(direct)     | n                           |
| Bytes(bytes)      | 4 o. 8                      |
| Container(direct) | n                           |
| Container(bytes)  | 4 o. 8                      |
| F32               | 1                           |
| F64               | 1                           |
| Bool              | 2                           |
| Unit              | 1                           |
+-------------------+-----------------------------+
| Summe             | 5 + 6(4|8 + n)              |
| Variante 1/2/4/8  | 29 + 6n => n = 37, 5 wasted |
| Variante 1-8      | 53 + 6n => n = 33, 5 wasted | => 11 mehr, dann n = 32 = 2^5, 0 wasted
+-------------------+-----------------------------+
| Nachricht current | 6x32 + 5 = 197, 59 wasted   |
| CBOR              |                             |
| Msgpack           | hot mess, 1 wasted          |
| Ion               | hot mess, 48 wasted         |
| RION              |                             |
+-------------------+-----------------------------+

## Tabellen

```
Vec<Struct>

( ($id: 1, $value: "foo"), ($id: 2, $value: "bar"), ($id: 3, $value: "error") )
( ($id: 1, $value: "foo"), (@1: 2, @2: "bar"), (@1: 3, @2: "error) )
( ($id, $value), (1, "foo"), (2, "bar"), (3, "error") )
( (1, "foo"), (2, "bar"), (3, "error") ) # nicht selbstbeschreibend!
```

## Container Länge

+-----------------+----------------+--------------------+
| Prozess         | Länge in Bytes | Länge in Elementen |
+-----------------+----------------+--------------------+
| Serialisieren   | Schwierig      | Einfach            |
| Deserialisieren | Schwierig      | Einfach            |
| Skippen         | Einfach        | Schwierig          |
+-----------------+----------------+--------------------+

Skippen wird unmöglich, wenn Symbole und Referenzen verwendet werden!
