# granny-rs

A Rust reimplementation of **Granny** (`com.dvloper.granny` 1.8.12, DVloper),
reconstructed from the shipped Android build.

## Provenance

Every value in this repository was recovered from the APK, not invented:

| Input | SHA-256 |
|---|---|
| `granny.xapk` | `b04850aea69047be06ca41439941bf35db5dd5d37fbbf347819a6b4cc806699d` |
| `com.dvloper.granny.apk` (base) | `b6783b0f6e934feceafc5e31214e1119f6421b6b7c3d58f3bb52e7008c5aab56` |
| `config.arm64_v8a.apk` | `54c2d24dfeba81d799922e4c8f9788ee28dd07930eda9ff2546386bc620bbe41` |
| `global-metadata.dat` (v31) | `03b2db2750c12fcd10038baaa2a63be7969cd6b907c24af5c8eb8a16503a67c4` |

- Engine: Unity **IL2CPP**, metadata v31
- 10,037 types · 65,995 methods · 40,942 fields · 67,315 parameters · 75 images
- `Assembly-CSharp.dll`: 885 types / 4,943 methods
- Scenes: `SplashScreen`, `Menu`, `Scene` (`level2`), `EndScene`

## What is reconstructed

**Game logic** — ported from the decompiled ARM64 in `libil2cpp.so`:

| Original | Rust |
|---|---|
| `EnemyAIGranny` (type 4020, 22 methods) | `game::Granny`, `World::granny_decisions` |
| `AIfollow` (4008) | `game::AiState` |
| `FPSControllerNEW` (4079) | `game::Player`, `World::tick` |
| `InventoryController` (4366) | `game::Item`, `World::inventory` |
| `StartNewDay` (4490) | `World::next_day`, `Difficulty::days` |
| `OpenCloseDoors` (4224) | `game::Door` state machine |

`EnemyAIGranny.GrannyDecisions` at `0x145b020` bails out when any of six busy
flags is set; those are reproduced verbatim as `AiBusy`.

**Assets** — extracted from the real APK:

- 359 meshes / 231,969 faces (`level2` scene geometry)
- 713 textures, packed into a 16×16 atlas
- 235 audio clips
- 2,299 submesh draws with correct per-submesh material assignment

## Build

```bash
cargo build --release
./target/release/granny-rs        # needs assets/scene.bin + assets/atlas.png
```

Android (arm64) is built in CI only — see `.github/workflows/build.yml`.

## Layout

```
src/scene.rs      GRSC pack loader (real Unity geometry)
src/render.rs     wgpu renderer + atlas sampling
src/game.rs       recovered game rules and AI
src/entities.rs   runtime entity meshes
assets/scene.bin  extracted level2 geometry + transforms
assets/atlas.png  extracted texture atlas
```

## Legal

Granny and its assets are © DVloper. This repository contains an independent
Rust implementation plus assets extracted from a copy of the game for
interoperability research. Do not redistribute the assets.

## Status

- `fmt` / `clippy -D warnings` / 11 behavioural tests: green
- Linux x86_64 release build: green
- Android arm64 APK: built in CI via `cargo ndk` + Gradle

See `docs/SPEC.md` for the method→Rust mapping and `docs/VERIFICATION.md` for
the evidence trail (metadata counts, RVAs, Unity asset rules).
