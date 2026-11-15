# pwg

`pwg` recomputes a password from a secret key, a site name, an account name, and a counter. No password is stored anywhere. The same input always gives the same password, so you only have to remember one key.

Written in Rust. One binary, no network, no daemon, no clipboard.

## How it works

1. You type the key at a prompt. It is not echoed, and it never goes through arguments or environment variables.
2. `pwg` normalizes the site and account names, then builds a salt from them and the counter.
3. The key and salt go through Argon2id. The result seeds a SHAKE256 byte stream.
4. The byte stream is mapped to characters according to the profile rules (length, character classes), without modulo bias.

The key is the only secret. The profile file is not a cryptographic secret, but it lists your sites and accounts, so treat it as private.

## Building

You need Rust (2021 edition). The project was built here with Rust 1.97.0.

```
cargo build --release
```

The binary is `target/release/pwg`.

`pwg --version` prints `pwg 0.1-<hash>`, where `<hash>` is the first 8 characters of the git commit the binary was built from. A build with uncommitted changes to tracked files gets a `-dirty` suffix. A build without git (for example from a zip) prints `0.1-unknown`. The hash identifies the build only. Passwords depend on the derivation spec below, not on the hash.

## Usage

```
pwg <action> [site] [flags]
```

There are four actions. The action is required, so `pwg github.com` fails with `ambiguous, which action?`.

| Action | Short | What it does |
|--------|-------|--------------|
| `gen` | `-g` | Compute and print a password |
| `add` | `-a` | Save a new profile |
| `bump` | `-b` | Raise a profile's counter by 1 |
| `list` | `-l` | Show saved profiles |

Example:

```
pwg add github.com --account budi@gmail.com --length 16 --require l1,u1,d1
pwg gen github.com
pwg bump github.com
pwg list
```

Main flags:

- `--account <a>`: account name or email.
- `--length <n>`: password length, 1 to 128 (default 20).
- `--require <spec>`: minimum characters per class, for example `l1,u1,d2,s1` (l lowercase, u uppercase, d digit, s symbol).
- `--symbols <s>`: allowed symbols (default `!@#$%^&*-_`).
- `--counter <n>`: for `gen`, applies to one call only and saves nothing.
- `--profile <path>`: use a different profile file.
- `--confirm`: ask for the key twice, so a typo is caught.
- `-v`, `--verbose`: print the profile path, final rules, and Argon2 parameters to stderr. The key, salt, and password are never printed.

`pwg -h` and `pwg <action> -h` show the full help.

### When a site asks you to change your password

1. `pwg gen github.com --counter 1` prints the old password.
2. Log in and change the password on the site.
3. `pwg bump github.com`.
4. `pwg gen github.com` prints the new password.

## Derivation spec v1

This part is frozen. Changing any step changes every password, so changes only happen through a new version (`pwg-v2`).

| Item | Value |
|------|-------|
| Context | `pwg-v1\|site\|account\|counter` |
| Salt | SHA3-256 of the context (32 bytes) |
| KDF | Argon2id, version 0x13, m = 65536 KiB, t = 3, p = 4, 32-byte output |
| Byte stream | SHAKE256 over the KDF output |
| Number selection | 2 bytes per attempt, biased tail rejected (rejection sampling) |
| Class order | lowercase, uppercase, digit, symbol |
| Shuffle | Fisher-Yates, from the last index down to 1 |

Normalization:

- Site: trim, strip the scheme (`https://`), cut at `/`, `?` or `#`, lowercase ASCII letters. `www.` is not stripped, so `www.github.com` and `github.com` are two different sites. Characters outside ASCII 0x21 to 0x7E, and `|`, are rejected.
- Account: trim and lowercase ASCII letters. Control characters and `|` are rejected.
- Key: the UTF-8 bytes exactly as typed. There is no Unicode normalization, so an ASCII passphrase is the safer choice.

## Profile file

JSON. Default locations:

- Linux, macOS, Termux: `$XDG_CONFIG_HOME/pwg/profiles.json`, or `$HOME/.config/pwg/profiles.json`.
- Windows: `%APPDATA%\pwg\profiles.json`.

On Unix the folder is created with mode 0700 and the file with mode 0600. Writes are atomic: the data goes to a `.tmp` file that is then renamed, so a crash does not leave a broken file. Unknown fields are rejected, so a typo like `lenght` is caught right away.

```json
{
  "version": 1,
  "profiles": [
    {
      "site": "github.com",
      "account": "budi@gmail.com",
      "length": 16,
      "require": { "lower": 1, "upper": 1, "digit": 1, "symbol": 0 },
      "symbols": "!@#$%^&*-_",
      "counter": 1
    }
  ]
}
```

## Argon2 memory and speed

These are estimates, not measurements on a 512 MB device.

- With m = 65536 KiB, Argon2 needs about 64 MiB for its memory matrix, plus a little program overhead. The 64 MiB figure comes straight from the parameter, not from a measurement. [Likely]
- From that figure, a device with 512 MB of RAM will probably cope if enough memory is free. This has not been tested on real hardware. [Likely]
- If the allocation fails, the process probably aborts instead of returning a clean error. Not checked. [Likely]
- Time per `gen` depends on the CPU. It has not been measured on any weak device, so no number of seconds is promised. [Guessing]
- The parameters cannot be lowered without changing every password.

Before using it for real accounts, run the golden tests on the weakest device you have.

## Security

What it protects:

- One site's password does not reveal another's, because the salt differs per site, account, and counter.
- There is no password database to breach.
- The key never appears in arguments, the environment, or shell history.

What it does not protect, and you have to accept:

- Key strength decides everything. If one password leaks and the attacker knows the site, they can guess the key offline. Argon2id slows the guessing but does not rescue a weak passphrase. Use a random passphrase of 8 to 10 words from a 7776-word list (dice or a generator), not words you picked yourself.
- A lost key means every password is lost. Keep a written backup in two safe places, with the exact format (`kuda-meja` and `kuda meja` are two different keys).
- The key cannot be fully wiped from memory. `Zeroizing` clears the buffer on drop, but it cannot guarantee that old copies left by reallocation are clean.
- A mistyped key is not detected. Use `--confirm` when registering a new account. A key verifier is deliberately not stored, because it would give an attacker a way to guess the key without a leaked password.
- The password is printed to the terminal and may stay in the scrollback.
- Site and account names are visible in `ps`. The key is not.

## Project layout

```
src/
  main.rs        thin entry point
  lib.rs         run() and execute()
  error.rs       single AppError enum
  cli/           argument parser (std only), validation, help text
  external/      wrappers for third-party crates
  profile/       rules, validation, profile read and write
  derive/        normalization and key derivation
  generate/      byte stream to password mapping
build.rs         computes the version string from git
tests/           integration tests and golden vectors
ref/ref.py       independent reference implementation (Python)
```

## Tests

```
cargo test
```

`tests/fixtures/golden.json` holds the golden vectors. `ref/ref.py` computes the same vectors with a separate Python implementation (needs `argon2-cffi`) so the results can be compared. Every crate upgrade must be followed by the golden tests, because passwords have to stay the same.

## Credits

`pwg` is built on the following Rust crates. Thanks to their authors.

| Crate | Used for | License |
|-------|----------|---------|
| [argon2](https://github.com/RustCrypto/password-hashes/tree/master/argon2) | Argon2id | MIT or Apache-2.0 |
| [sha3](https://github.com/RustCrypto/hashes) | SHA3-256 and SHAKE256 | MIT or Apache-2.0 |
| [rpassword](https://github.com/conradkleinespel/rpassword) | Key prompt without echo | Apache-2.0 |
| [zeroize](https://github.com/RustCrypto/utils) | Clearing key buffers | Apache-2.0 or MIT |
| [serde](https://github.com/serde-rs/serde) | Profile serialization | MIT or Apache-2.0 |
| [serde_json](https://github.com/serde-rs/json) | Profile JSON format | MIT or Apache-2.0 |

The licenses in the table were read from each crate's `Cargo.toml` metadata, downloaded at build time (versions in `Cargo.lock`).

The Argon2 parameters follow the second (memory-constrained) option in RFC 9106. The numbers were taken from a secondary quotation, not from the RFC itself.

## License

MIT. See [LICENSE](LICENSE). Copyright 2026 niluminun.
