#!/usr/bin/env bash
# Build skycot/marketplace-browser. Tags <git-sha> + :latest; optional
# --push-to <registry> after the build. See README.md alongside.
set -euo pipefail

IMAGE_NAME="skycot/marketplace-browser"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOCKERFILE="${REPO_ROOT}/images/marketplace-browser/Dockerfile"

usage() {
    cat <<'EOF'
build.sh — build skycot/marketplace-browser locally.

  --push-to <registry>  Push to <registry>/marketplace-browser
                        (requires prior `docker login <registry>`).
  --help, -h            Show this help.

Every build applies two tags: :<short-git-sha> and :latest.
EOF
}

PUSH_TARGET=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --push-to)
            shift
            [[ $# -gt 0 ]] || { echo "error: --push-to needs a registry" >&2; exit 2; }
            PUSH_TARGET="$1"; shift ;;
        --help|-h) usage; exit 0 ;;
        *) echo "error: unknown argument '$1' (try --help)" >&2; exit 2 ;;
    esac
done

if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker not found on PATH. Install Docker first." >&2
    echo "  macOS: https://docs.docker.com/desktop/install/mac-install/" >&2
    echo "  Linux: https://docs.docker.com/engine/install/" >&2
    exit 1
fi
if ! docker --version >/dev/null 2>&1; then
    echo "error: 'docker --version' failed; check daemon reachability" >&2
    exit 1
fi

GIT_SHA="$(git -C "${REPO_ROOT}" rev-parse --short HEAD 2>/dev/null || echo unknown)"
SHA_TAG="${IMAGE_NAME}:${GIT_SHA}"
LATEST_TAG="${IMAGE_NAME}:latest"

echo "==> building ${SHA_TAG} (and ${LATEST_TAG}) from ${DOCKERFILE}"
docker build -f "${DOCKERFILE}" -t "${SHA_TAG}" -t "${LATEST_TAG}" "${REPO_ROOT}"

if [[ -n "${PUSH_TARGET}" ]]; then
    REMOTE_SHA="${PUSH_TARGET}/marketplace-browser:${GIT_SHA}"
    REMOTE_LATEST="${PUSH_TARGET}/marketplace-browser:latest"
    echo "==> tagging + pushing to ${PUSH_TARGET}"
    docker tag "${SHA_TAG}" "${REMOTE_SHA}"
    docker tag "${LATEST_TAG}" "${REMOTE_LATEST}"
    docker push "${REMOTE_SHA}"
    docker push "${REMOTE_LATEST}"
fi

echo "done. local tags: ${SHA_TAG}, ${LATEST_TAG}"
[[ -n "${PUSH_TARGET}" ]] && echo "      pushed:     ${PUSH_TARGET}/marketplace-browser"
exit 0
