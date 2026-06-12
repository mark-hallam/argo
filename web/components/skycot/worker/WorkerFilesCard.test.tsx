import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { WorkerFilesCard } from "@/components/skycot/worker/WorkerFilesCard";
import { ApiError } from "@/lib/api/types";

// Mock the network surface; keep `formatBytes` real, stub `fileToBase64`
// (jsdom's FileReader data-URL path is flaky) so the upload flow is
// deterministic.
vi.mock("@/lib/api/worker-files", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/worker-files")>(
      "@/lib/api/worker-files",
    );
  return {
    ...actual,
    listWorkerFiles: vi.fn(),
    uploadWorkerFile: vi.fn(),
    deleteWorkerFile: vi.fn(),
    fileToBase64: vi.fn(async () => "ZA=="),
  };
});

vi.mock("@/lib/skycot/demo-mode", () => ({
  hasRealSession: vi.fn(() => true),
}));

import {
  listWorkerFiles,
  uploadWorkerFile,
  deleteWorkerFile,
} from "@/lib/api/worker-files";
import { hasRealSession } from "@/lib/skycot/demo-mode";

const listMock = vi.mocked(listWorkerFiles);
const uploadMock = vi.mocked(uploadWorkerFile);
const deleteMock = vi.mocked(deleteWorkerFile);
const sessionMock = vi.mocked(hasRealSession);

function listing(files: { name: string; size_bytes: number }[]) {
  return {
    files,
    total_bytes: files.reduce((a, f) => a + f.size_bytes, 0),
    quota_bytes: 100 * 1024 * 1024,
    max_file_bytes: 10 * 1024 * 1024,
  };
}

beforeEach(() => {
  sessionMock.mockReturnValue(true);
  listMock.mockResolvedValue(listing([]));
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerFilesCard", () => {
  it("mock-gates: signed-out shows a sign-in notice and never fetches", async () => {
    sessionMock.mockReturnValue(false);
    render(<WorkerFilesCard workerId="wkr-1" />);
    expect(await screen.findByTestId("worker-files-signin")).toBeInTheDocument();
    expect(listMock).not.toHaveBeenCalled();
  });

  it("renders the honest empty state when the folder is empty", async () => {
    render(<WorkerFilesCard workerId="wkr-1" />);
    expect(await screen.findByTestId("worker-files-empty")).toBeInTheDocument();
    expect(listMock).toHaveBeenCalledWith("wkr-1");
  });

  it("lists file names + sizes, never contents", async () => {
    listMock.mockResolvedValue(
      listing([{ name: "invoice.txt", size_bytes: 2048 }]),
    );
    render(<WorkerFilesCard workerId="wkr-1" />);
    const row = await screen.findByTestId("worker-files-row");
    expect(row).toHaveTextContent("invoice.txt");
    expect(row).toHaveTextContent("2.0 KB");
  });

  it("uploads the picked file then reloads the listing", async () => {
    uploadMock.mockResolvedValue({ filename: "notes.md", size_bytes: 5 });
    // Second list call (post-upload) returns the new file.
    listMock
      .mockResolvedValueOnce(listing([]))
      .mockResolvedValueOnce(listing([{ name: "notes.md", size_bytes: 5 }]));
    render(<WorkerFilesCard workerId="wkr-1" />);
    await screen.findByTestId("worker-files-empty");

    const input = screen.getByTestId("worker-files-input") as HTMLInputElement;
    const file = new File(["d"], "notes.md", { type: "text/markdown" });
    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() =>
      expect(uploadMock).toHaveBeenCalledWith("wkr-1", "notes.md", "ZA=="),
    );
    expect(await screen.findByText("notes.md")).toBeInTheDocument();
  });

  it("surfaces the gateway's honest error on a failed upload", async () => {
    uploadMock.mockRejectedValue(
      new ApiError(413, "file exceeds 10485760-byte limit", "payload_too_large"),
    );
    render(<WorkerFilesCard workerId="wkr-1" />);
    await screen.findByTestId("worker-files-empty");

    const input = screen.getByTestId("worker-files-input") as HTMLInputElement;
    const file = new File(["d"], "big.txt", { type: "text/plain" });
    fireEvent.change(input, { target: { files: [file] } });

    const err = await screen.findByTestId("worker-files-error");
    expect(err).toHaveTextContent("file exceeds 10485760-byte limit");
  });

  it("deletes a file via the row trash button", async () => {
    deleteMock.mockResolvedValue(undefined);
    listMock
      .mockResolvedValueOnce(listing([{ name: "a.txt", size_bytes: 5 }]))
      .mockResolvedValueOnce(listing([]));
    render(<WorkerFilesCard workerId="wkr-1" />);
    const del = await screen.findByTestId("worker-files-delete");
    fireEvent.click(del);
    await waitFor(() =>
      expect(deleteMock).toHaveBeenCalledWith("wkr-1", "a.txt"),
    );
  });
});
