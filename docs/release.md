# Releases

The GitHub Actions release workflow runs only by manual dispatch. It builds an
aarch64 macOS binary and x86_64 and aarch64 Linux musl binaries. The workflow
checks that each Linux binary has no ELF interpreter or shared-library
dependency before packaging it as a zip file and creating a pre-release named
`release-<commit>`.

To build locally, use `make release`. Both `make release` and
`make verify-release` accept a `TARGET` override. The defaults and release
matrix are defined in the [Makefile](../Makefile) and [release workflow](../.github/workflows/release.yml).
