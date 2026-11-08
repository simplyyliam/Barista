# 0003: Generate TypeScript types with specta

Status: accepted (2026-11-08, v0.0 Setup)

## Context

The protocol is defined once in Rust (`crates/protocol`) and the SDK needs the same types in
TypeScript (`packages/sdk/src/protocol.ts`). The bar's Tauri commands need the same thing for
`apps/bar/src/shared/bindings.ts`. Options for a plain Rust crate:

|                                | specta (+ specta-serde, specta-typescript) | ts-rs           | typeshare             |
| ------------------------------ | ------------------------------------------ | --------------- | --------------------- |
| Latest                         | 2.0.0-rc.26 (release candidate)            | 12.0.1 (stable) | 1.0.5 (stable)        |
| Reads serde attributes         | yes (specta-serde)                         | yes             | partly                |
| Same tool as `bindings.ts`     | **yes** (tauri-specta is built on it)      | no              | no                    |
| Needs an extra CLI             | no, runs in `cargo test`                   | no              | yes (`typeshare-cli`) |
| Other languages (C# SDK later) | other exporters exist, early               | no              | yes                   |

## Decision

Use **specta**. `crates/protocol` derives `specta::Type` and a test (`export_protocol_types`)
writes `protocol.ts`; tauri-specta does the same for `bindings.ts`. `pnpm gen` runs both tests
(`cargo test --workspace export_`), and CI fails if `git diff` shows the committed files were stale.

- One derive, one output style, one tool for both generated files.
- specta has no Tauri dependency, so the protocol crate stays a plain crate.
- tauri-specta pins `specta =2.0.0-rc.26`, so we pin `specta`, `specta-serde` and
  `specta-typescript` to the matching exact versions in every `Cargo.toml`.

## Consequences

- specta 2 is still a release candidate: its API can change. Exact pins mean upgrades are a
  deliberate step (bump all pinned crates together, run `pnpm gen`, read the diff).
- If specta breaks the protocol export, the fallback is ts-rs 12 for `crates/protocol` only.
  The SDK would not notice: it imports the generated `protocol.ts` either way.
- Revisit when the C# SDK (v1.2+) needs the protocol in C#.
