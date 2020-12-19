# nachricht-nq

Easily create human-readable representations of nachricht messages. Just pipe any nachricht-producing call to `nq` and
get something you can make sense of in an instant: where you would write `curl api.example.com/resource -H 'Accept:
application/json' | jq` you can also write `curl api.example.com/resource -H 'Accept: application/x-nachricht' | nq`.

```bash
echo -en "\x62\x01\x02" | nq
(
  true,
  false,
)
```

