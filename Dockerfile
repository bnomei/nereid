# syntax=docker/dockerfile:1

ARG NEREID_VERSION=0.9.0
ARG NEREID_REPOSITORY=bnomei/nereid
ARG NEREID_RUNTIME_IMAGE=gcr.io/distroless/static-debian13:nonroot

FROM --platform=$BUILDPLATFORM alpine:3.22 AS fetch
ARG NEREID_VERSION
ARG NEREID_REPOSITORY
ARG TARGETARCH

RUN apk add --no-cache ca-certificates curl

RUN set -eux; \
  case "$TARGETARCH" in \
    amd64) target=x86_64-unknown-linux-musl ;; \
    arm64) target=aarch64-unknown-linux-musl ;; \
    *) echo "unsupported Docker target architecture: $TARGETARCH" >&2; exit 1 ;; \
  esac; \
  tag="v${NEREID_VERSION#v}"; \
  archive="nereid-${tag}-${target}.tar.gz"; \
  url="https://github.com/${NEREID_REPOSITORY}/releases/download/${tag}/${archive}"; \
  curl -fsSL -o "/tmp/${archive}" "$url"; \
  curl -fsSL -o "/tmp/${archive}.sha256" "${url}.sha256"; \
  cd /tmp; \
  sha256sum -c "${archive}.sha256"; \
  tar -xzf "$archive"; \
  chmod 755 nereid; \
  mkdir -p /tmp/nereid-workspace

FROM ${NEREID_RUNTIME_IMAGE}
ARG NEREID_VERSION

ENV HOME=/workspace

LABEL org.opencontainers.image.title="Nereid"
LABEL org.opencontainers.image.description="Source-available noncommercial terminal diagram TUI and MCP server for Mermaid-backed sessions"
LABEL org.opencontainers.image.source="https://github.com/bnomei/nereid"
LABEL org.opencontainers.image.licenses="LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives"
LABEL org.opencontainers.image.version="${NEREID_VERSION}"

COPY --from=fetch --chown=65532:65532 /tmp/nereid /usr/local/bin/nereid
COPY --from=fetch --chown=65532:65532 /tmp/nereid-workspace /workspace

WORKDIR /workspace
USER 65532:65532

ENTRYPOINT ["/usr/local/bin/nereid"]
