/**
 * Worker file-ingress API client (brief
 * `launch-cloud-file-ingress-upload-to-fs-root`).
 *
 * Backs the worker-detail "Files" control: list / upload / delete files
 * inside a teammate's `fs_root` (the folder the teammate's fenced
 * `file_read` / `file_write` work against). On a cloud host this is the
 * only way a customer puts a file in front of their teammate, so the
 * proven value loop depends on it.
 *
 * Honest by construction: every call hits the real gateway
 * (`apiGet`/`apiPost`/`apiDelete`); a non-2xx surfaces as the gateway's
 * `{ error: { code, message } }` envelope via `ApiError`. There is NO
 * mock-success fallback — file ingress that didn't really land must read
 * as a failure, not a fabricated "uploaded".
 */

import { apiDelete, apiGet, apiPost } from "@/lib/api/client";

/** One file inside a teammate's `fs_root` — name + size only, never
 *  contents. */
export interface WorkerFile {
  name: string;
  size_bytes: number;
}

/** Response shape of `GET /api/v1/workers/<id>/files`. */
export interface WorkerFilesListing {
  files: WorkerFile[];
  total_bytes: number;
  quota_bytes: number;
  max_file_bytes: number;
}

/** Result of a successful upload. */
export interface WorkerFileUploaded {
  filename: string;
  size_bytes: number;
}

/** List the files in the worker's `fs_root`. */
export function listWorkerFiles(workerId: string): Promise<WorkerFilesListing> {
  return apiGet<WorkerFilesListing>(
    `/api/v1/workers/${encodeURIComponent(workerId)}/files`,
  );
}

/**
 * Upload one file into the worker's `fs_root`. The gateway carries the
 * whole request as UTF-8 text, so the bytes ride as base64 in JSON (the
 * only lossless ingress shape for binary content on this host).
 */
export function uploadWorkerFile(
  workerId: string,
  filename: string,
  contentBase64: string,
): Promise<WorkerFileUploaded> {
  return apiPost<WorkerFileUploaded>(
    `/api/v1/workers/${encodeURIComponent(workerId)}/files`,
    { filename, content_base64: contentBase64 },
  );
}

/** Delete one file from the worker's `fs_root`. */
export function deleteWorkerFile(
  workerId: string,
  name: string,
): Promise<void> {
  return apiDelete<void>(
    `/api/v1/workers/${encodeURIComponent(workerId)}/files/${encodeURIComponent(name)}`,
  );
}

/**
 * Read a browser `File` into a base64 string (no `data:` prefix). Pure
 * over the FileReader API so the upload control can hand the gateway the
 * `content_base64` field directly.
 */
export function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error ?? new Error("file read failed"));
    reader.onload = () => {
      const result = typeof reader.result === "string" ? reader.result : "";
      // `data:<mime>;base64,<payload>` → keep only the payload.
      const comma = result.indexOf(",");
      resolve(comma >= 0 ? result.slice(comma + 1) : result);
    };
    reader.readAsDataURL(file);
  });
}

/** Human-readable byte size for the control's size labels. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
