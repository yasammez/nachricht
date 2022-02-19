# nachricht-nq

```bash
cargo install nachricht-nq
```

Transform nachricht messages between the wire format and the textual representation. By default, `nq` will treat input
as binary and generate textual output. This can be used to peek into a program's output with ease.

```bash
echo -en "\x82\x01\x02" | nq
[
  true,
  false,
]
```

The `-t` switch can be used to treat input as textual form instead. This is useful to format a message on the fly.

```bash
echo "[true,false]" | nq -t
(
  true,
  false,
)
```

The `-e` switch will produce the output in the wire format. This is useful to canonicalize inefficiently encoded
messages or within a pipe to verify the data's validity.

```bash
echo -en "\x2f\x00\x00\x00\x00\x00\x00\x00\x02" | nq -e | hexdump -v -e '/1 "%02x "'; echo
22
```

The two switches can also be combined to generate the wire format from the textual representation. This is useful to
quickly feed a nachricht-expecting program some data from the command line.

```bash
echo "[true,false]" | nq -te | hexdump -v -e '/1 "%02x "'; echo
82 01 02
```

Finally, you can edit any nachricht encoded file with the `-f <PATH>` option. This will open the file in a temporary
buffer in your default editor to make changes within the textual representation.

```bash
echo -en "\x82\x01\x02" > nachricht.nch
nq -f nachricht.nch
```
