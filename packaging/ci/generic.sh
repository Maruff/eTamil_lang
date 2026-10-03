#!/bin/sh
# A pipeline for any CI that can run a shell: Jenkins, Bitbucket, Drone, a cron
# job, a pre-push hook. Copy ci/install-etamil.sh and ci/check-qmz.sh into your
# repository, then make this the job's script.
set -eu

sh ci/install-etamil.sh "${ETAMIL_VERSION:-latest}"
export PATH="$HOME/.local/bin:$PATH" ETAMIL_PATH="$HOME/.local/lib/etamil"

sh ci/check-qmz.sh src          # errors only, nothing runs
etamil src/main.qmz             # replace with your own entry point
