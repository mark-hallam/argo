# Doctor Summary Mode

`argo doctor --summary` runs the normal doctor checks and prints one
line for scripts and dashboards:

```text
OK 13/13 checks
DEGRADED 12/13 checks (1 warn: gateway)
FAIL 12/13 checks (1 fail: audit)
```

Exit codes are `0` for `OK`, `1` for `DEGRADED`, and `2` for `FAIL`.
Use `argo doctor --summary --json` for the compact envelope:

```json
{"state":"OK","passed":13,"warned":0,"failed":0,"issue":null}
```
