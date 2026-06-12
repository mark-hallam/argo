"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { FileUp, Trash2, AlertCircle, FolderOpen } from "lucide-react";
import { Card, CardBody, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { hasRealSession } from "@/lib/skycot/demo-mode";
import { ApiError } from "@/lib/api/types";
import {
  deleteWorkerFile,
  fileToBase64,
  formatBytes,
  listWorkerFiles,
  uploadWorkerFile,
  type WorkerFile,
} from "@/lib/api/worker-files";

/**
 * Files control on the worker-detail page (brief
 * `launch-cloud-file-ingress-upload-to-fs-root`).
 *
 * Lets the owner drop a file into the teammate's `fs_root` so the next
 * run can read it — the cloud ingress for the proven fenced-file value
 * loop. LOCKED BRAND tokens only; honest errors (a failed upload reads
 * as failure, never a fabricated success).
 *
 * **Mock-gated**: file ingress writes to a real on-host folder, so it is
 * meaningless without a real signed-in session. In the anonymous /
 * demo-showcase path we render an explanatory disabled state instead of
 * firing a doomed fetch — the honest mock-gate (no fake file list).
 */
export function WorkerFilesCard({ workerId }: { workerId: string }) {
  const [signedIn, setSignedIn] = useState(false);
  const [files, setFiles] = useState<WorkerFile[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // `hasRealSession()` reads `document.cookie`, so it only resolves
  // post-mount (server render has no session marker).
  useEffect(() => {
    setSignedIn(hasRealSession());
  }, []);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const listing = await listWorkerFiles(workerId);
      setFiles(listing.files);
    } catch (e) {
      setError(messageFor(e, "Could not load files."));
    } finally {
      setLoading(false);
    }
  }, [workerId]);

  useEffect(() => {
    if (!signedIn) {
      setLoading(false);
      return;
    }
    void reload();
  }, [signedIn, reload]);

  const onPick = useCallback(
    async (file: File | undefined) => {
      if (!file) return;
      setBusy(true);
      setError(null);
      try {
        const b64 = await fileToBase64(file);
        await uploadWorkerFile(workerId, file.name, b64);
        await reload();
      } catch (e) {
        setError(messageFor(e, "Upload failed."));
      } finally {
        setBusy(false);
        if (inputRef.current) inputRef.current.value = "";
      }
    },
    [workerId, reload],
  );

  const onDelete = useCallback(
    async (name: string) => {
      setBusy(true);
      setError(null);
      try {
        await deleteWorkerFile(workerId, name);
        await reload();
      } catch (e) {
        setError(messageFor(e, "Delete failed."));
      } finally {
        setBusy(false);
      }
    },
    [workerId, reload],
  );

  return (
    <Card data-testid="worker-files-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <FolderOpen className="size-4 text-[var(--primary)]" aria-hidden />
          Files
        </CardTitle>
        <CardDescription>
          Drop a file into this teammate&rsquo;s working folder. It lands inside
          their fenced <code>fs_root</code>, so the next run can read it.
        </CardDescription>
      </CardHeader>
      <CardBody className="space-y-4">
        {!signedIn ? (
          <p
            data-testid="worker-files-signin"
            className="text-sm text-[var(--muted-foreground)]"
          >
            Sign in to upload files to your teammate&rsquo;s folder.
          </p>
        ) : (
          <>
            <div className="flex items-center gap-3">
              <input
                ref={inputRef}
                type="file"
                data-testid="worker-files-input"
                className="hidden"
                onChange={(e) => void onPick(e.target.files?.[0])}
              />
              <Button
                type="button"
                variant="primary"
                loading={busy}
                disabled={busy}
                onClick={() => inputRef.current?.click()}
              >
                <FileUp className="size-4" aria-hidden />
                Upload file
              </Button>
            </div>

            {error ? (
              <p
                role="alert"
                data-testid="worker-files-error"
                className="flex items-center gap-2 text-sm text-[var(--destructive)]"
              >
                <AlertCircle className="size-4 shrink-0" aria-hidden />
                {error}
              </p>
            ) : null}

            {loading ? (
              <p className="text-sm text-[var(--muted-foreground)]">Loading…</p>
            ) : files.length === 0 ? (
              <p
                data-testid="worker-files-empty"
                className="text-sm text-[var(--muted-foreground)]"
              >
                No files yet. Upload one to get your teammate started.
              </p>
            ) : (
              <ul className="divide-y divide-[var(--border)] rounded-md border border-[var(--border)]">
                {files.map((f) => (
                  <li
                    key={f.name}
                    data-testid="worker-files-row"
                    className="flex items-center justify-between gap-3 px-3 py-2"
                  >
                    <span className="truncate text-sm text-[var(--foreground)]">
                      {f.name}
                    </span>
                    <span className="flex items-center gap-3">
                      <span className="text-xs text-[var(--muted-foreground)]">
                        {formatBytes(f.size_bytes)}
                      </span>
                      <button
                        type="button"
                        aria-label={`Delete ${f.name}`}
                        data-testid="worker-files-delete"
                        disabled={busy}
                        onClick={() => void onDelete(f.name)}
                        className="text-[var(--muted-foreground)] hover:text-[var(--destructive)] disabled:opacity-50"
                      >
                        <Trash2 className="size-4" aria-hidden />
                      </button>
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
      </CardBody>
    </Card>
  );
}

/** Pull the gateway's honest error message out of an `ApiError`, else a
 *  generic fallback. Never invents a success. */
function messageFor(e: unknown, fallback: string): string {
  if (e instanceof ApiError) {
    // The client parses the gateway's `{ error: { code, message } }`
    // envelope into `.message`; surface it verbatim (honest error).
    return e.message || `${fallback} (${e.status})`;
  }
  return fallback;
}
