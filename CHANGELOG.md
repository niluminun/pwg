# Changelog

## [0.1.0] - 2026-11-15

### Added
- Actions `gen`, `add`, `bump`, `list` with short forms `-g`, `-a`, `-b`, `-l`.
- Derivation spec `pwg-v1`: Argon2id (m=65536 KiB, t=3, p=4), SHA3-256 salt, SHAKE256 byte stream.
- JSON profile file with atomic writes and private file modes on Unix.
- `--version` prints `pwg 0.1-<hash>` (8-character git commit, `-dirty` for uncommitted changes).
- Prebuilt binaries for Linux (x86_64, aarch64; static, musl), Windows (x86_64) and macOS (aarch64, x86_64), with SHA256SUMS and build attestations.

### Password impact
- None. This is the first release; spec `pwg-v1` is frozen.
