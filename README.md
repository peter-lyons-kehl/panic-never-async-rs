# panic-never-async-rs

## Goal

- `panic-never-async-rs` is a demo of
   [japaric/panic-never](https://github.com/japaric/panic-never) for `async`
   Rust.
- it's incorporated in [peter-lyons-kehl/panic-never-rs ->
  examples/](https://github.com/peter-lyons-kehl/panic-never-rs/tree/master/examples)
- see also
  [japaric/panic-never#3](https://github.com/japaric/panic-never/pull/3)
- `panic-never` itself is a build time-only validation, and its simplest use
   makes executables that run indefinitely (no exit code because of `no_std`).
   This demo shows how to make such executables return/exit (on Linux only).
