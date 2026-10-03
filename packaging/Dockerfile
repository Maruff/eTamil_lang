# The official eTamil image: the same static musl package the release workflow
# publishes, unpacked on Alpine. Nothing is compiled here, so a multi-arch build
# is fast and the image holds exactly the binary people download elsewhere.
#
#   bash packaging/build.sh                      # makes dist/etamil-linux-<arch>.tar.gz
#   docker build -f packaging/Dockerfile -t etamil .
#   docker run --rm etamil --version
#   docker run --rm -v "$PWD:/work" etamil program.qmz
#
# The archives must already be in dist/. build.sh builds only the current
# machine's architecture, so a local build is single-arch; the release workflow
# supplies both.
#
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

FROM alpine:3.21

ARG TARGETARCH

# A bind mount rather than COPY: the archives are used and never enter a layer,
# so the image does not carry them.
RUN --mount=type=bind,source=dist,target=/dist \
    set -eu; \
    case "$TARGETARCH" in \
        amd64) arch=x64 ;; \
        arm64) arch=arm64 ;; \
        *) echo "unsupported architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    tar -xzf "/dist/etamil-linux-$arch.tar.gz" -C /tmp; \
    pkg="/tmp/etamil-linux-$arch"; \
    install -m 0755 "$pkg/etamil" /usr/local/bin/etamil; \
    mkdir -p /usr/local/lib/etamil; \
    cp -r "$pkg/nUlakam" "$pkg/examples" /usr/local/lib/etamil/; \
    rm -rf "$pkg"; \
    adduser -D -h /work etamil

# Lets இறக்கு "nUlakam/..." resolve from any directory, as install.sh does.
ENV ETAMIL_PATH=/usr/local/lib/etamil

LABEL org.opencontainers.image.title="eTamil" \
      org.opencontainers.image.source="https://github.com/Maruff/eTamil_lang" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later"

USER etamil
WORKDIR /work
ENTRYPOINT ["etamil"]
