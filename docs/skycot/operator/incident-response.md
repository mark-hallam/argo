# Incident response — operator runbook

## Export session for support

When you need to hand a session to Skycot support (or an internal SRE) for offline analysis, use `argo replay export` to capture a self-contained bundle.

```bash
# Full-fidelity bundle (includes secrets — handle the file accordingly).
argo replay export 01J9XYZ... --out /tmp/session-01J9XYZ.tar.gz

# Sanitized bundle — provider keys, JWTs, emails, phone numbers redacted.
argo replay export 01J9XYZ... --out /tmp/session-01J9XYZ.sanitized.tar.gz --sanitize

# Uncompressed tar (useful when the destination is already compressed,
# e.g. a tarball-of-tarballs incident bundle).
argo replay export 01J9XYZ... --out /tmp/session-01J9XYZ.tar --uncompressed
```

### Bundle layout

```
session-<id>.tar.gz
├── manifest.toml        # session metadata, sha256, sanitized flag, audit_row_count
├── audit/
│   ├── chain.jsonl      # full audit rows in seq order (canonical JSON, one per line)
│   └── verify.txt       # `argo audit verify-chain` summary captured at export time
├── skills/<id>.json     # attached skill ids + attach timestamp
├── bundle/action-<seq>.json  # per-tool-call request/result payloads
└── README.md            # short replay instructions
```

`manifest.toml` records `sha256 = "..."` — the SHA-256 of every bundled file's bytes in sorted-path order. A mismatch on re-hash means the bundle was tampered with after export.

### Replay on the receiving side

```bash
# Extract into an inspection directory.
mkdir -p session-replay && tar xzf session-01J9XYZ.tar.gz -C session-replay/

# Re-verify the audit chain offline.
argo audit verify-chain --from-file session-replay/audit/chain.jsonl
```

### Sanitization details

`--sanitize` walks every JSON entry in the bundle and replaces string values matching:

- Provider keys: `sk-…` (Anthropic / OpenAI style).
- JWTs: `eyJ…` 3-segment tokens.
- Email addresses (RFC-5322-ish).
- Phone numbers (E.164-ish).

Replacements are deterministic: each redacted value becomes `<label>-redacted-<hmac>`, where the hmac is a short prefix of the SHA-256 of the original value. Same input → same placeholder, so cross-referencing redacted bundles still works.

Field names and JSON structure are preserved; only string values are touched.

### Caveats

- Memory rows are not yet exported. The `memory/` directory is reserved but currently unpopulated — a follow-up brief wires the per-session memory walk.
- Full-fidelity (non-sanitized) bundles contain whatever the session captured, including secrets and PII. Treat the file like the live database.
