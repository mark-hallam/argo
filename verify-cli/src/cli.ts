#!/usr/bin/env node
/**
 * `skycot-verify <bundle.zip>` — offline integrity verifier.
 *
 * Exit 0 on full pass; exit 1 on any failure (hash chain mismatch,
 * signature mismatch, malformed bundle, missing entries).
 */

import { resolve } from "node:path";

import { verifyBundle } from "./verify";
import { VerifyError, type VerifyResult } from "./types";

// Resolved relative to the compiled `dist/cli.js`. In the published
// npm layout both `dist/` and `src/keys/` are shipped (see the
// `files` list in package.json), so this path resolves the same way
// for `npm install -g` users and for local `node dist/cli.js` runs.
const DEFAULT_KEY_PATH = resolve(__dirname, "..", "src", "keys", "public-key.pem");

interface ParsedArgs {
  bundlePath?: string;
  publicKeyPath: string;
  help: boolean;
  version: boolean;
}

function parseArgs(argv: string[]): ParsedArgs {
  const out: ParsedArgs = {
    publicKeyPath: DEFAULT_KEY_PATH,
    help: false,
    version: false,
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "-h" || arg === "--help") {
      out.help = true;
    } else if (arg === "-V" || arg === "--version") {
      out.version = true;
    } else if (arg === "--public-key") {
      const value = argv[++i];
      if (!value) throw new Error("--public-key requires a path argument");
      out.publicKeyPath = resolve(value);
    } else if (arg.startsWith("--public-key=")) {
      out.publicKeyPath = resolve(arg.slice("--public-key=".length));
    } else if (arg.startsWith("-")) {
      throw new Error(`unknown option: ${arg}`);
    } else if (!out.bundlePath) {
      out.bundlePath = resolve(arg);
    } else {
      throw new Error(`unexpected positional argument: ${arg}`);
    }
  }
  return out;
}

function usage(): string {
  return [
    "Usage: skycot-verify <bundle.zip> [--public-key <path>]",
    "",
    "Verifies the integrity of a SKYCOT audit evidence bundle:",
    "  - Recomputes the SHA-256 hash chain over canonical actions.",
    "  - Verifies the Ed25519 signature against the shipped public key.",
    "",
    "Exit codes:",
    "  0  full pass (hash chain matches, signature valid)",
    "  1  any failure",
    "",
    "Options:",
    "  --public-key <path>   override the shipped public key",
    "  -h, --help            show this message",
    "  -V, --version         show version",
  ].join("\n");
}

function renderReport(result: VerifyResult): string {
  const lines: string[] = [];
  lines.push(`Bundle: ${result.bundle}`);
  lines.push(`Format: ${result.format}`);
  lines.push(`Actions verified: ${result.actionsVerified}`);
  lines.push(
    result.hashChainOk
      ? "Hash chain: ✓ matches"
      : `Hash chain: ✗ ${result.hashChainError ?? "mismatch"}`
  );
  if (result.signatureOk) {
    const meta = signerMeta(result);
    lines.push(meta ? `Signature:  ✓ valid (${meta})` : "Signature:  ✓ valid");
  } else {
    lines.push(`Signature:  ✗ ${result.signatureError ?? "invalid"}`);
  }
  return lines.join("\n");
}

function signerMeta(result: VerifyResult): string | undefined {
  const parts: string[] = [];
  if (result.signedAt) parts.push(`signed ${result.signedAt}`);
  if (result.signedBy) parts.push(`by ${result.signedBy}`);
  return parts.length > 0 ? parts.join(" ") : undefined;
}

export async function run(
  argv: string[],
  stdout: NodeJS.WritableStream,
  stderr: NodeJS.WritableStream
): Promise<number> {
  let args: ParsedArgs;
  try {
    args = parseArgs(argv);
  } catch (err) {
    stderr.write(`skycot-verify: ${(err as Error).message}\n`);
    stderr.write(usage() + "\n");
    return 1;
  }

  if (args.help) {
    stdout.write(usage() + "\n");
    return 0;
  }
  if (args.version) {
    const pkg = require("../package.json") as { version: string };
    stdout.write(`${pkg.version}\n`);
    return 0;
  }
  if (!args.bundlePath) {
    stderr.write("skycot-verify: missing bundle path\n");
    stderr.write(usage() + "\n");
    return 1;
  }

  try {
    const result = await verifyBundle({
      bundlePath: args.bundlePath,
      publicKeyPath: args.publicKeyPath,
    });
    stdout.write(renderReport(result) + "\n");
    return result.hashChainOk && result.signatureOk ? 0 : 1;
  } catch (err) {
    if (err instanceof VerifyError) {
      stderr.write(`skycot-verify: ${err.message}\n`);
    } else {
      stderr.write(`skycot-verify: unexpected error: ${(err as Error).message}\n`);
    }
    return 1;
  }
}

if (require.main === module) {
  run(process.argv.slice(2), process.stdout, process.stderr).then((code) => {
    process.exit(code);
  });
}
