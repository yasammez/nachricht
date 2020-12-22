# nachricht-nq

```bash
cargo install nachricht-nq
```

Transform nachricht messages between the wire format and the textual representation. By default, `nq` will treat input
as binary and generate textual output. This can be used to peek into a program's output with ease.

```bash
echo -en "\x62\x01\x02" | nq
(
  true,
  false,
)
```

The `-t` switch can be used to treat input as textual form instead. This is useful to format a message on the fly.

```bash
echo "(true,false)" | nq -t
(
  true,
  false,
)
```

The `-e` switch will produce the output in the wire format. This is useful to canonicalize inefficiently encoded
messages or within a pipe to verify the data's validity.

```bash
echo -en "\x7f\x00\x00\x00\x00\x00\x00\x00\x02\x01\x02" | nq -e | hexdump -v -e '/1 "%02x "'; echo
62 01 02
```

Finally, the two switches can be combined to generate the wire format from the textual representation. This is useful to
quickly feed a nachricht-expecting program some data from the command line.

```
echo "(true,false)" | nq -te | hexdump -v -e '/1 "%02x "'; echo
62 01 02
```
