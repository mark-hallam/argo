#!/usr/bin/env bash
# post-install.sh — printed after `brew install argo` or after the
# user mounts the DMG. Hints at the next-step `argo onboard` flow.
set -e

cat <<'EOF'

  argo is installed. Quick next steps:

    1. Run the first-experience walkthrough:
           argo onboard

    2. Optional — install the launchd agent so the gateway starts
       on login (writes ~/Library/LaunchAgents/com.skycot.argo.plist):
           argo daemon install

    3. Verify the install:
           argo doctor

  Documentation: https://github.com/mark-hallam/Argo-Agent

EOF
