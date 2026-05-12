#!/usr/bin/env bash
# SKYCOT single-VM installer (Ubuntu 24.04 LTS). See deploy/vm/README.md.
#
# Idempotent: each step records a marker under /var/lib/argo/.install-state/
# so re-runs skip completed work (<30s on a healthy host). All output is
# teed to /var/log/argo-install.log for forensics.
#
# Usage: sudo bash deploy/vm/install.sh [--with-tls <domain>] [--update]
#                                       [--dry-run] [--use-release]

set -euo pipefail

ARGO_HOME=/var/lib/argo
ARGO_CONFIG_DIR=/etc/argo
ARGO_STATE_DIR=${ARGO_HOME}/.install-state
ARGO_REPO_DIR=/opt/argo
ARGO_LOG=/var/log/argo-install.log
ARGO_USER=argo
ARGO_PG_USER=argo
ARGO_PG_DB=argo
ARGO_GATEWAY_PORT=8080
ARGO_WEB_PORT=3000

DRY_RUN=0; USE_RELEASE=0; UPDATE_ONLY=0; TLS_DOMAIN=
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "${SCRIPT_DIR}/../.." && pwd)

while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY_RUN=1 ;;
        --use-release) USE_RELEASE=1 ;;
        --update) UPDATE_ONLY=1 ;;
        --with-tls) TLS_DOMAIN=${2:-}; shift ;;
        -h|--help) sed -n '2,9p' "$0"; exit 0 ;;
        *) echo "unknown flag: $1" >&2; exit 2 ;;
    esac
    shift
done

log() { printf '[%s] %s\n' "$(date -u +%FT%TZ)" "$*"; }
die() { log "FATAL: $*" >&2; exit 1; }
# shellcheck disable=SC2294  # eval is intentional: run() accepts compound shell strings
run() { if [ "$DRY_RUN" = 1 ]; then echo "+ $*"; else eval "$@"; fi; }
have_marker() { [ -f "${ARGO_STATE_DIR}/$1" ]; }
mark() { [ "$DRY_RUN" = 1 ] && return 0; mkdir -p "${ARGO_STATE_DIR}"; touch "${ARGO_STATE_DIR}/$1"; chown -R "${ARGO_USER}:${ARGO_USER}" "${ARGO_STATE_DIR}" 2>/dev/null || true; }

if [ "$DRY_RUN" = 0 ]; then
    [ "$(id -u)" = 0 ] || { echo "must run as root (use sudo)" >&2; exit 1; }
    mkdir -p "$(dirname "$ARGO_LOG")"
    exec > >(tee -a "$ARGO_LOG") 2>&1
fi

step_check_os() {
    log "[check_os] verifying Ubuntu 24.04 LTS"
    [ "$DRY_RUN" = 1 ] && return 0
    local id ver
    # shellcheck source=/dev/null
    id=$(. /etc/os-release && echo "${ID:-}")
    # shellcheck source=/dev/null
    ver=$(. /etc/os-release && echo "${VERSION_ID:-}")
    [ "$id" = "ubuntu" ] && [ "$ver" = "24.04" ] \
        || die "Ubuntu 24.04 required; got ${id} ${ver}"
}

step_install_packages() {
    if have_marker packages && [ "$UPDATE_ONLY" = 0 ]; then
        log "[packages] already installed, skipping"; return 0
    fi
    log "[packages] apt update + install base packages"
    run "DEBIAN_FRONTEND=noninteractive apt-get update -y"
    run "DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        build-essential pkg-config libssl-dev libsqlite3-dev curl git \
        nginx postgresql-16 postgresql-contrib certbot python3-certbot-nginx \
        nodejs npm ca-certificates openssl"
    mark packages
}

step_create_user() {
    if have_marker user; then
        log "[user] argo system user exists, skipping"
        return 0
    fi
    log "[user] creating argo system user with home ${ARGO_HOME}"
    if [ "$DRY_RUN" = 0 ] && ! id -u "${ARGO_USER}" >/dev/null 2>&1; then
        useradd --system --create-home --home-dir "${ARGO_HOME}" \
            --shell /usr/sbin/nologin "${ARGO_USER}"
    fi
    run "mkdir -p ${ARGO_HOME} ${ARGO_CONFIG_DIR} ${ARGO_HOME}/backups"
    run "chown -R ${ARGO_USER}:${ARGO_USER} ${ARGO_HOME}"
    mark user
}

step_install_postgres() {
    if have_marker postgres; then
        log "[postgres] already initialized, skipping"
        return 0
    fi
    log "[postgres] enabling service + provisioning role/db"
    run "systemctl enable --now postgresql"
    if [ "$DRY_RUN" = 0 ]; then
        local pg_pw
        pg_pw=$(openssl rand -hex 24)
        sudo -u postgres psql -tAc "SELECT 1 FROM pg_roles WHERE rolname='${ARGO_PG_USER}'" \
            | grep -q 1 || sudo -u postgres psql -c \
            "CREATE ROLE ${ARGO_PG_USER} LOGIN PASSWORD '${pg_pw}'"
        sudo -u postgres psql -tAc "SELECT 1 FROM pg_database WHERE datname='${ARGO_PG_DB}'" \
            | grep -q 1 || sudo -u postgres createdb -O "${ARGO_PG_USER}" "${ARGO_PG_DB}"
        umask 077
        printf 'localhost:5432:%s:%s:%s\n' "${ARGO_PG_DB}" "${ARGO_PG_USER}" "${pg_pw}" \
            > "${ARGO_HOME}/.pgpass"
        chown "${ARGO_USER}:${ARGO_USER}" "${ARGO_HOME}/.pgpass"
        chmod 0400 "${ARGO_HOME}/.pgpass"
        echo "${pg_pw}" > "${ARGO_CONFIG_DIR}/.pg_password"
        chmod 0400 "${ARGO_CONFIG_DIR}/.pg_password"
    fi
    mark postgres
}

step_generate_env() {
    if have_marker env && [ "$UPDATE_ONLY" = 0 ]; then
        log "[env] /etc/argo/argo.env exists, skipping"
        return 0
    fi
    log "[env] writing /etc/argo/{argo,web}.env"
    if [ "$DRY_RUN" = 0 ]; then
        local bearer pg_pw
        bearer=$(openssl rand -hex 32)
        pg_pw=$(cat "${ARGO_CONFIG_DIR}/.pg_password" 2>/dev/null || echo "")
        umask 077
        cat > "${ARGO_CONFIG_DIR}/argo.env" <<EOF
ARGO_GATEWAY_BEARER_TOKEN=${bearer}
DATABASE_URL=postgres://${ARGO_PG_USER}:${pg_pw}@127.0.0.1:5432/${ARGO_PG_DB}
ARGO_HOME=${ARGO_HOME}
ARGO_TENANT_DEFAULT=default
ARGO_LOG_LEVEL=info
EOF
        cat > "${ARGO_CONFIG_DIR}/web.env" <<EOF
NODE_ENV=production
NEXT_TELEMETRY_DISABLED=1
PORT=${ARGO_WEB_PORT}
NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:${ARGO_GATEWAY_PORT}
EOF
        chmod 0640 "${ARGO_CONFIG_DIR}"/*.env
        chown root:"${ARGO_USER}" "${ARGO_CONFIG_DIR}"/*.env
    fi
    mark env
}

step_fetch_source() {
    if have_marker source && [ "$UPDATE_ONLY" = 0 ]; then
        log "[source] ${ARGO_REPO_DIR} present, skipping"
        return 0
    fi
    log "[source] syncing ${ARGO_REPO_DIR}"
    if [ -d "${REPO_ROOT}/.git" ] && [ "${REPO_ROOT}" != "${ARGO_REPO_DIR}" ]; then
        run "rm -rf ${ARGO_REPO_DIR}"
        run "cp -a ${REPO_ROOT} ${ARGO_REPO_DIR}"
    fi
    if [ "$UPDATE_ONLY" = 1 ] && [ -d "${ARGO_REPO_DIR}/.git" ]; then
        run "git -C ${ARGO_REPO_DIR} pull --ff-only"
    fi
    mark source
}

step_build_argo() {
    if [ "$USE_RELEASE" = 1 ]; then
        die "--use-release is reserved for the future signed-release artifact path"
    fi
    if have_marker argo_binary && [ "$UPDATE_ONLY" = 0 ]; then
        log "[build_argo] /usr/local/bin/argo present, skipping"
        return 0
    fi
    log "[build_argo] cargo build --release --bin argo (5-10 min on 4GB VMs)"
    if [ "$DRY_RUN" = 0 ]; then
        if ! command -v cargo >/dev/null 2>&1; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
                | sh -s -- -y --default-toolchain 1.75.0 --profile minimal
            # shellcheck disable=SC1091
            . "$HOME/.cargo/env"
        fi
        ( cd "${ARGO_REPO_DIR}" && cargo build --release --bin argo )
        install -m 0755 "${ARGO_REPO_DIR}/target/release/argo" /usr/local/bin/argo
    fi
    mark argo_binary
}

step_build_web() {
    if have_marker web_build && [ "$UPDATE_ONLY" = 0 ]; then
        log "[build_web] web build present, skipping"
        return 0
    fi
    log "[build_web] npm ci + npm run build"
    if [ "$DRY_RUN" = 0 ]; then
        run "cp -a ${ARGO_REPO_DIR}/web ${ARGO_HOME}/web"
        chown -R "${ARGO_USER}:${ARGO_USER}" "${ARGO_HOME}/web"
        sudo -u "${ARGO_USER}" bash -c "cd ${ARGO_HOME}/web && npm ci && NEXT_TELEMETRY_DISABLED=1 npm run build"
    fi
    mark web_build
}

step_install_units() {
    log "[units] copying systemd units + nginx config"
    run "install -m 0644 ${SCRIPT_DIR}/argo.service /etc/systemd/system/argo.service"
    run "install -m 0644 ${SCRIPT_DIR}/web.service /etc/systemd/system/web.service"
    run "install -m 0644 ${SCRIPT_DIR}/nginx-skycot.conf /etc/nginx/sites-available/skycot"
    if [ "$DRY_RUN" = 0 ] && [ -f /etc/nginx/sites-enabled/default ]; then
        rm -f /etc/nginx/sites-enabled/default
    fi
    run "ln -sf /etc/nginx/sites-available/skycot /etc/nginx/sites-enabled/skycot"
    run "systemctl daemon-reload"
    mark units
}

step_enable_services() {
    log "[services] enabling + starting argo, web, nginx"
    run "systemctl enable --now argo.service web.service nginx"
    if [ "$UPDATE_ONLY" = 1 ]; then
        run "systemctl restart argo.service web.service"
    fi
    mark services
}

step_setup_tls() {
    [ -n "$TLS_DOMAIN" ] || { log "[tls] no --with-tls; skipping (HTTP only)"; return 0; }
    log "[tls] running certbot --nginx for ${TLS_DOMAIN}"
    run "install -m 0644 ${SCRIPT_DIR}/certbot-renew.cron /etc/cron.d/skycot-certbot"
    run "certbot --nginx --non-interactive --agree-tos --redirect \
        --email admin@${TLS_DOMAIN} -d ${TLS_DOMAIN}"
    mark tls
}

step_smoke_test() {
    log "[smoke] curl localhost gateway + web"
    [ "$DRY_RUN" = 1 ] && return 0
    sleep 5
    curl -fsS "http://127.0.0.1:${ARGO_GATEWAY_PORT}/api/v1/health" >/dev/null \
        || die "gateway health check failed; see journalctl -u argo"
    curl -fsS "http://127.0.0.1:${ARGO_WEB_PORT}/" >/dev/null \
        || die "web health check failed; see journalctl -u web"
    log "[smoke] OK"
}

step_print_summary() {
    [ "$DRY_RUN" = 1 ] && return 0
    local bearer
    bearer=$(grep -E '^ARGO_GATEWAY_BEARER_TOKEN=' "${ARGO_CONFIG_DIR}/argo.env" | cut -d= -f2- || echo "")
    cat <<EOF

=========================================================
 SKYCOT single-VM install complete
=========================================================
 Gateway URL : http://localhost:${ARGO_GATEWAY_PORT}/api/v1/health
 Web URL     : http://localhost:${ARGO_WEB_PORT}/
 Public URL  : ${TLS_DOMAIN:+https://${TLS_DOMAIN}}${TLS_DOMAIN:-http://<this-host>}/
 Bearer      : ${bearer}
 Postgres pw : stored at ${ARGO_HOME}/.pgpass (0400, argo:argo)
 Logs        : journalctl -u argo -u web -f
 Install log : ${ARGO_LOG}
=========================================================
EOF
}

main() {
    log "starting installer (dry_run=${DRY_RUN} update=${UPDATE_ONLY} tls=${TLS_DOMAIN:-none})"
    step_check_os
    step_install_packages
    step_create_user
    step_install_postgres
    step_generate_env
    step_fetch_source
    step_build_argo
    step_build_web
    step_install_units
    step_enable_services
    step_setup_tls
    step_smoke_test
    step_print_summary
}

main "$@"
