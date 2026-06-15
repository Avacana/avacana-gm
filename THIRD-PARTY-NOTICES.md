# Third-Party Notices

`avacana-gm` itself is licensed under the [MIT License](LICENSE).

To provide native Git operations it links against — and, with the default build
options, statically bundles — several third-party components distributed under their
own licenses. This file summarizes the ones that carry obligations beyond a simple
permissive license. It is provided for convenience and is not legal advice.

## Native / vendored libraries

### libgit2

- **License:** GPL-2.0 **WITH a linking exception**.
- **Version:** 1.9.2, pulled in via the `git2` → `libgit2-sys` crates with the
  `vendored-libgit2` feature (the C sources are compiled and statically linked).
- **What the exception grants:** unlimited permission to link the compiled library
  into combinations with other programs — including proprietary or MIT-licensed
  software — and to distribute those combinations without the GPL extending to your
  own code. This is why an MIT crate may link libgit2.
- **Obligations that remain:**
  - If you **redistribute compiled artifacts** that include libgit2, convey a copy
    of its license and make the corresponding libgit2 source available on request.
  - If you **modify libgit2 itself**, the modified library must be distributed under
    GPL-2.0 — the exception only covers linking the *unmodified* library.
- **Source & full license text:** <https://github.com/libgit2/libgit2> — see its
  `COPYING` file, which also lists the sub-components libgit2 itself bundles (zlib,
  llhttp, and, on some platforms, PCRE) together with their license texts.

### OpenSSL

- **License:** Apache-2.0 (OpenSSL 3.x).
- **Version:** 3.x, pulled in via `openssl-sys` / `openssl-src` when the
  `vendored-openssl` feature is enabled. Apache-2.0 is compatible with both MIT and
  libgit2's GPL-2.0-with-exception.
- **Source & license:** <https://github.com/openssl/openssl>

## Rust dependencies

The remaining dependencies are permissive (MIT / Apache-2.0 / BSD / ISC / Zlib /
Unicode-3.0), with one weak-copyleft exception — `option-ext` (MPL-2.0, reached via
`dirs`) — whose obligations are limited to its own source files.

License policy for the dependency graph is enforced by `cargo deny check licenses`
(see `deny.toml`). A complete, machine-generated attribution list — including the
full license text of every crate — can be produced with
[`cargo about`](https://crates.io/crates/cargo-about) or
[`cargo bundle-licenses`](https://crates.io/crates/cargo-bundle-licenses).

> Note: `cargo deny` and `cargo about` read each crate's declared `license`
> metadata, not the C sources vendored inside `libgit2-sys`. libgit2's GPL-2.0
> therefore does not surface in those tools' output — it is documented here instead.
