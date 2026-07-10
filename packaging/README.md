# Nereid Packaging

Nereid's release assets are the source of truth for binary installers:

- `npx @bnomei/nereid` is published from `npm/nereid`; the wrapper downloads the matching GitHub Release asset and verifies its `.sha256`.
- `docker run ghcr.io/bnomei/nereid:<version>` is built from `Dockerfile`, which downloads the musl Linux release asset for the target image architecture.
- Homebrew formula metadata lives in the sibling `homebrew-nereid` repository.

The npm publish job is skipped unless the release environment provides `NPM_TOKEN`.

Linux release assets use musl targets so the published binaries are portable static
executables. The Docker image therefore consumes:

- `x86_64-unknown-linux-musl` for `linux/amd64`
- `aarch64-unknown-linux-musl` for `linux/arm64`

The default runtime base is `gcr.io/distroless/static-debian13:nonroot`.

Default Docker image assets:

```bash
VERSION=0.9.0 TARGET=x86_64-unknown-linux-musl scripts/build-release.sh
VERSION=0.9.0 TARGET=x86_64-unknown-linux-musl scripts/package-release.sh

VERSION=0.9.0 TARGET=aarch64-unknown-linux-musl scripts/build-release.sh
VERSION=0.9.0 TARGET=aarch64-unknown-linux-musl scripts/package-release.sh
```

Use `cross` for the aarch64 musl target unless the matching musl C toolchain is
installed on the host.
