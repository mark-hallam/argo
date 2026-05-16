#!/usr/bin/env bash
# Capture product screenshots from the SKYCOT SPA for docs + marketing.
#
# Drives Playwright (Chromium) against a local Next.js dev server, walks
# every scene in docs/skycot/img/manifest.json, captures desktop
# 1920x1080 and mobile 360x800 PNGs per scene, converts to WebP via
# cwebp (quality 78, target <200 KB), and writes to BOTH
# docs/skycot/img/screenshots/ and marketing/public/screenshots/.
#
# Prereqs (one-time):  brew install webp
#                      cd web && pnpm install
#                      cd web && pnpm exec playwright install chromium
#
# Usage:  ./scripts/capture-product-screenshots.sh
#         ./scripts/capture-product-screenshots.sh --scene approval-card
#         ./scripts/capture-product-screenshots.sh --keep-server
#
# The SPA is launched with NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 and the
# gateway URL pointed at a non-listening port so the client falls back
# to bundled sample data — no live backend required.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB_DIR="${REPO_ROOT}/web"
MANIFEST="${REPO_ROOT}/docs/skycot/img/manifest.json"
DOC_OUT="${REPO_ROOT}/docs/skycot/img/screenshots"
MKT_OUT="${REPO_ROOT}/marketing/public/screenshots"
TMP_DIR="$(mktemp -d -t skycot-screenshots-XXXXXX)"

SCENE_FILTER="" KEEP_SERVER=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --scene) SCENE_FILTER="$2"; shift 2 ;;
    --keep-server) KEEP_SERVER=1; shift ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

command -v cwebp >/dev/null 2>&1 || { echo "FAIL: cwebp not on PATH (brew install webp)" >&2; exit 1; }
command -v pnpm  >/dev/null 2>&1 || { echo "FAIL: pnpm not on PATH" >&2; exit 1; }
[[ -f "${MANIFEST}" ]] || { echo "FAIL: manifest not found at ${MANIFEST}" >&2; exit 1; }

mkdir -p "${DOC_OUT}" "${MKT_OUT}"

SERVER_PID=""
cleanup() { rm -rf "${TMP_DIR}" "${WEB_DIR}/capture.mjs"; [[ -n "${SERVER_PID}" ]] && kill "${SERVER_PID}" 2>/dev/null || true; }
trap cleanup EXIT

if [[ ${KEEP_SERVER} -eq 0 ]]; then
  echo ">> launching next dev on :3000"
  ( cd "${WEB_DIR}" \
    && NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:1 \
       pnpm dev >"${TMP_DIR}/dev.log" 2>&1 ) &
  SERVER_PID=$!
  for _ in $(seq 1 60); do
    code=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000 || echo 000)
    [[ "${code}" =~ ^(200|307|308)$ ]] && { echo ">> dev server up"; break; }
    sleep 2
  done
fi

SPEC="${WEB_DIR}/capture.mjs"
cat >"${SPEC}" <<'JS'
import { chromium } from "@playwright/test";
import fs from "node:fs";
const manifest = JSON.parse(fs.readFileSync(process.env.MANIFEST, "utf8"));
const filter = process.env.SCENE_FILTER || "";
const outDir = process.env.PNG_OUT;
const base = "http://localhost:3000";
const browser = await chromium.launch();
try {
  for (const scene of manifest.scenes) {
    if (filter && scene.id !== filter) continue;
    for (const [size, dims] of Object.entries(manifest.viewports)) {
      const ctx = await browser.newContext({ viewport: dims, deviceScaleFactor: 1 });
      const page = await ctx.newPage();
      await page.goto(base + scene.route, { waitUntil: "networkidle" }).catch(() => {});
      await page.addStyleTag({ content: "* { caret-color: transparent !important; animation: none !important; transition: none !important; } nextjs-portal, [data-nextjs-toast], [data-next-mark-loading], #__next-build-watcher { display: none !important; }" });
      await page.waitForTimeout(500);
      const out = `${outDir}/${scene.id}.${size}.png`;
      await page.screenshot({ path: out, fullPage: false });
      console.log(`captured ${out}`);
      await ctx.close();
    }
  }
} finally { await browser.close(); }
JS

echo ">> running Playwright capture"
PNG_OUT="${TMP_DIR}/png"; mkdir -p "${PNG_OUT}"
( cd "${WEB_DIR}" \
  && MANIFEST="${MANIFEST}" SCENE_FILTER="${SCENE_FILTER}" PNG_OUT="${PNG_OUT}" \
     node --experimental-vm-modules "${SPEC}" ) 2>&1 | sed 's/^/    /'

echo ">> converting PNG -> WebP (q=78, target <200 KB)"
shopt -s nullglob
for png in "${PNG_OUT}"/*.png; do
  base=$(basename "${png}" .png)
  webp_doc="${DOC_OUT}/${base}.webp"
  cwebp -quiet -q 78 -m 6 "${png}" -o "${webp_doc}"
  q=78
  while [[ $(stat -f%z "${webp_doc}" 2>/dev/null || stat -c%s "${webp_doc}") -gt 204800 && ${q} -gt 40 ]]; do
    q=$((q - 8))
    cwebp -quiet -q ${q} -m 6 "${png}" -o "${webp_doc}"
  done
  cp "${webp_doc}" "${MKT_OUT}/${base}.webp"
done

echo ">> done."
ls -lah "${DOC_OUT}" | tail -n +2 | awk '{printf "    %s  %s\n", $5, $9}'
