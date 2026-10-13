# Releases

The GitHub Actions release workflow runs only by manual dispatch. It builds an
aarch64 macOS binary and x86_64 and aarch64 Linux musl binaries. Linux builds
include static and dynamic variants; the workflow verifies both before
packaging them as zip files and creating a pre-release named `release-<commit>`.

To build locally, use `make release`. For a dynamic Linux musl binary, use
`make release-dynamic`. Both accept a `TARGET` override; `make verify-release`
and `make verify-release-dynamic` check the corresponding Linux artifacts. The
defaults and release matrix are defined in the [Makefile](../Makefile) and
[release workflow](../.github/workflows/release.yml).
