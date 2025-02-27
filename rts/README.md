The Motoko RTS (C and Rust parts)
=================================

This directory contains the parts of the Motoko runtime implemented in Rust.

tl;dr
-----

If you just want to get `mo-rts.wasm` in this directory, run

    nix-shell --run 'make -C rts'

from the top-level directory of the Motoko repository.

Compilation
-----------

Running `make` should produce `mo-rts.wasm`.

If run within `nix-shell`, the environment variables `WASM_CLANG` and `WASM_LD`
should point to suitable binaries (we track a specific unreleased version of
`llvm`). If not present, the `Makefile` will try to use `clang-10` and
`wasm-ld-10`.

The runtime compiles and links in [libtommath]. It needs the source, so
`nix-build` and `nix-shell` will set the environment variable `TOMMATHSRC` to
point to the source in `/nix/store`.

If not present, the `Makefile` will look in `../../libtommath`, i.e. parallel
to the `motoko` repository; this is useful if you need to hack on libtommath.

[libtommath]: https://github.com/libtom/libtommath

Exporting and importing functions
---------------------------------

Import and export as if you are importing from or exporting to a C library. Examples:

```rust
// Expects bigint_trap to be provided at link time. The function should follow
// C calling conventions
extern "C" {
    fn bigint_trap() -> !;
}

// Provides bigint_add function. The function follows C calling conventions
#[no_mangle]
extern "C" fn bigint_add(...) { ... }
```

libtommath and memory management
--------------------------------

We have to make libtommath’s memory management (which expects functions
`alloc`, `calloc` and `realloc`) work with the Motoko runtime.
See `motoko-rts/src/bigint.rs` for the technical details.

Rust build
----------

The Rust parts are built from `motoko-rts`, using `xargo` and `cargo`.

To build this in nix, we need pre-fetch some dependencies (currently
`compiler_builtins` and `libc`). This works in `nix-build` by:

 * Adding`compiler_builtins` as a dependency in `Cargo.toml` (even though not
   needed), so that it shows up with a hash in `Cargo.lock`

 * Building a directory with vendored sources in `default.nix` (see `rustDeps`)

 * Configuring `cargo` to use that vendored directory (see `preBuild`)

If you change dependencies (e.g. bump versions, add more crates),

 1. Add them to `Cargo.toml`
 2. Make sure that `Cargo.lock` is up to date
 3. In `default.nix`, invalidate the `sha256` of `rustDeps` (e.g. change one
    character)
 4. Run `nix-build -A rts`. You should get an error message about the actual
    checksum.
 5. Set that as `sha256` of `rustDeps` in `default.nix`

Warning: nix will happily use a stale version of the dependencies if you do not
do step 3.

Running RTS tests
-----------------

- Build tests using rustc WASI target: `cargo build --target=wasm32-wasi`
- Run with wasmtime: `wasmtime target/wasm32-wasi/debug/motoko-rts-tests.wasm`

Debugging the RTS
-----------------

It is possible to build the RTS and test suite for i686 and debug using native
debug tools like gdb and rr. You first need to build tommath-related files.
This is easiest to do in `nix-shell`:

- (in `rts/`) `make _build/libtommath_i686.a` (this step requires headers and
  libraries for the target)
- (in `rts/`) `make _build/tommath_bindings.rs` (this step requires `bindgen`)

After these you can build the test suite for `i686-unknown-linux-gnu` target
outside of `nix-shell`. If you don't have the target installed already, install with

- `rustup +nightly target install i686-unknown-linux-gnu`

Now build i686 executable:

- (in `rts/motoko-rts-tests`) `cargo +nightly build --target=i686-unknown-linux-gnu`

Now you should see an i686 executable
`rts/motoko-rts-tests/target/i686-unknown-linux-gnu/debug/motoko-rts-tests`
that you can debug with e.g. `gdb`.

Ideally all of these steps would be done in `nix-shell` or outside, but the
last command does not work in `nix-shell` because of missing i686 libraries and
I couldn't figure out how to install those in nix.
