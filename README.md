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

## TODO

* ser/de sollten Value nicht referenzieren
* Don't repeat yourself in ser/de implementieren
* Slicezugriffe schöner machen (ReadIO-Trait wie in CBOR?)
* Dokumentieren
* Doctests
* Readme schreiben
* Lizenz wählen
* Fehler verbessern
* Fehlerpfade testen
* Container gibt Länge in Bytes statt Elementen an?
* Menschenlesbare Repräsentation parsen und rendern
* Serialisierung von Enums verbessern: braucht man so viele Container?
* nq: Prettyprinting
* nq: Parsing
* nq: Escaping
* nq: Darstellung von Keys: # & @ ' ! $ ...
