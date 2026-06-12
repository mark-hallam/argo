# deploy/smoke — LAST PASS

Auto-written by `deploy/smoke/run.sh` on every green run. Do NOT edit by
hand — it is the recorded evidence that the smoke stack actually came up
and served its endpoints on a real machine (DEC-029: "authored + builds"
is not "works").

- **Last green run (UTC):** 2026-06-12T07:53:10Z
- **Host:** Darwin 25.5.0 arm64
- **Gateway image (argo:dev) id:** `sha256:e9e9ace1f040970d91a4d636d69dddc9fbe911644b30759c086f5299fa793efb`
- **Web image (argo-web:dev) id:** `sha256:bba5847d47280eb20583f1c483b50b73575129e6ff8b3ce2b823e59d7a9abe20`
- **Gateway URL polled:** http://localhost:8080/health (200)
- **SPA URL polled:** http://localhost:3001/ (200)

## checks.ts output (all `"ok":true`)

```json
{"name":"health","ok":true,"detail":"status=ok"}
{"name":"spa","ok":true,"detail":"bytes=99598"}
{"name":"hire_flow","ok":true,"detail":"hire=400 approve=400"}
{"name":"metrics","ok":true,"detail":"bytes=14315"}
{"name":"summary","ok":true,"detail":"4/4 passed"}
```
