-- Stream `claude-filesystem-executor` (V1.2) — per-worker
-- filesystem sandbox root.
--
-- Adds the `fs_root` column to `worker`. Nullable for
-- backwards-compatibility — existing workers without an
-- ExecutorKind::Filesystem action just leave it NULL. New workers
-- spawned with `argo worker create --fs-root <path>` persist the
-- caller-supplied path; the FilesystemExecutor consults it at
-- dispatch time and refuses any op whose resolved path escapes.

ALTER TABLE worker ADD COLUMN fs_root TEXT;
