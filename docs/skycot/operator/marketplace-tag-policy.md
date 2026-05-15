# Marketplace Tag Policy

SKYCOT reads OCI registry tags when marketplace skills are discovered
from `~/.argo/marketplace.toml`. When a repository has more than one
tag, the scanner resolves exactly one manifest for the catalog row.

The default policy is `timestamp_newest`: the scanner reads candidate
manifests and selects the tag with the newest
`org.opencontainers.image.created` annotation. This matches the common
operator expectation that pushing a newer tag makes it the visible
marketplace entry.

Per-registry override:

```toml
[[registry]]
url = "https://registry.example.com"
namespace = "skills"
tag_policy = "timestamp_newest"
```

Supported values:

- `timestamp_newest` selects the newest manifest creation timestamp.
- `alphabetical_last` preserves the legacy deterministic tag ordering.
- Any other string selects that tag exactly, such as `tag_policy = "v1.2.3"`.

Use an explicit tag for pinned release channels and
`alphabetical_last` only when maintaining compatibility with an
existing registry naming scheme.
