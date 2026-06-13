# Line-ending policy (`win-line-endings-gitattributes-pin`)

**Goal:** make CRLF unable to silently corrupt byte-sensitive files on a Windows
checkout/commit, so a Windows working tree is byte-identical to a macOS/Linux
one. CRLF was hypothesised as a failure cause 3× during the Windows hardening
session — it never actually was (each was a real bug or a stale literal), but
the hypothesis recurring is the signal to **pin the class shut**.

## What's byte-sensitive (and why CRLF would break it)

- **Embedded SQL migrations** (`crates/*/migrations/*.sql`, 110 files) —
  `include_str!`'d into the binary and applied verbatim; a stray CR changes the
  bytes the migration runner sees.
- **Golden / fixture data** (`*.json`, `*.jsonl`, `*.csv`, `*.snap`) — compared
  byte-for-byte or hashed (e.g. audit-chain fixtures); a CR flips the compare.
- **Source with inline literals** (`*.rs`, …) — inline SQL / golden string
  literals embedded in Rust would carry a CR on a CRLF checkout.

## The pin

[`.gitattributes`](../../../.gitattributes) (repo root) forces these classes to
**LF in both the repo and the working tree** (`text eol=lf`), and marks true
binaries (`*.png`, `*.pdf`, `*.woff2`, `*.db`, …) `binary` so git never touches
their bytes. Covered text classes: `sql, jsonl, json, toml, csv, snap, rs, sh,
ps1, py, js, mjs, cjs, ts, tsx, jsx, css, scss, html, md, yml, yaml, txt, lock`.

**Deliberately surgical:** no repo-wide `* text=auto eol=lf` default is set.
Every tracked file matched by the rules is *already* LF today
(`git ls-files --eol` reports exactly one CRLF file in the whole repo —
`web/public/logos/calendly.svg`, a vendored web logo, which is **not** matched
by the rules and is left untouched). So adding `.gitattributes` introduces
**zero content renormalisation** — it is purely a forward guard. (If a future
brief wants the strongest possible policy, promote to `* text=auto eol=lf` and
renormalise that one SVG in the same commit.)

## Verification (on real Windows, x86_64-pc-windows-msvc)

```
$ git check-attr -a crates/argo-store/migrations/100__audit_worker_deleted_at.sql
  ...: text: set
  ...: eol: lf

# CRLF round-trip — the actual protection: a CRLF .sql is normalised to LF in
# the index on `git add` (i.e. a Windows commit can't smuggle CRLF into a
# byte-sensitive file):
$ printf 'SELECT 1;\r\nSELECT 2;\r\n' > probe.sql
$ git add probe.sql
  warning: in the working copy of 'probe.sql', CRLF will be replaced by LF ...
$ git ls-files --eol probe.sql
  i/lf    w/crlf  attr/text eol=lf    probe.sql      # ← stored as LF in the repo

# And no existing file is renormalised by adding the policy:
$ git status --short
  ?? .gitattributes
```

✅ The pin applies to migration SQL (`eol: lf`), actively normalises a CRLF
byte-sensitive file to LF on commit, and changes no existing committed bytes.

## Known pre-existing exception

`web/public/logos/calendly.svg` is the single tracked file committed with CRLF.
It is a vendored web logo (not embedded in the binary, not hashed), is not
matched by the pin, and is left as-is to keep this change content-neutral. A
follow-up may normalise it if/when the policy is promoted repo-wide.

_Verified by the Windows fleet (Latitude5440)._
