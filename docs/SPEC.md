# Recovered specification

Mapping from the shipped IL2CPP build to this Rust implementation.
Every address below is an RVA into `libil2cpp.so` and every type/method index
is a row in `global-metadata.dat` v31.

## Build identity

| | |
|---|---|
| package | `com.dvloper.granny` |
| version | 1.8.12 (code 92) |
| min / target SDK | 24 / 36 |
| engine | Unity IL2CPP, metadata v31 |
| metadata | 10,037 types · 65,995 methods · 40,942 fields · 67,315 params · 75 images |
| `Assembly-CSharp.dll` | 885 types · 4,943 methods |

## Type → Rust

| Type (index) | Methods | Rust |
|---|---|---|
| `EnemyAIGranny` (4020) | 22 @ 30260 | `game::Granny`, `World::granny_decisions` |
| `AIfollow` (4008) | 13 @ 30186 | `game::AiState` |
| `FPSControllerNEW` (4079) | 10 @ 30547 | `game::Player`, `World::tick` |
| `InventoryController` (4366) | 8 @ 31826 | `game::Item`, `World::inventory` |
| `StartNewDay` (4490) | 12 @ 32373 | `World::next_day` |
| `playerCaught` (4423) | 8 @ 32066 | `World::caught` |
| `playerDead` (4432) | 8 @ 32119 | `World::escaped` |
| `OpenCloseDoors` (4224) | 7 @ 31153 | `game::Door` |
| `BearTrap` (4287) | 5 @ 31526 | `Item::BearTrap` |
| `CrowControl` (4014) | 6 @ 30236 | `Item::BirdSeed` |
| `PlayerFreeze` (4434) | 5 @ 32161 | `AiBusy::frozen` |

## Key methods disassembled

| Method | RVA | Finding |
|---|---|---|
| `EnemyAIGranny.Start` | `0x1453e28` | 22-method AI bootstrap |
| `EnemyAIGranny.FixedUpdate` | `0x14549e4` | per-tick driver |
| `EnemyAIGranny.GrannyDecisions` | `0x145b020` | early-out on six flags at `+0x2c1..+0x2c7` |
| `EnemyAIGranny.Playercaught` | `0x145bacc` | catch sequence |
| `EnemyAIGranny.followPlayer` | `0x145bb60` | pursuit |
| `AIfollow.Update` | `0x144f1e4` | Slendrina/Mom follow |
| `FPSControllerNEW.FixedUpdate` | `0x14e4268` | movement integration |
| `FPSControllerNEW.PlayerGetsCaught` | `0x14e6ae0` | death handoff |
| `InventoryController.CheckInventory` | `0x151f828` | item gating |
| `StartNewDay.Start` | `0x154e1a4` | day loop |
| `OpenCloseDoors.Update` | `0x1500d38` | door animation |
| `PickUp.Update` | `0x152a938` | proximity pickup |

## Recovered constants

| Constant | Value | Source |
|---|---|---|
| day length | 300 s | `StartNewDay` timer budget |
| walk / run / crouch | 2.6 / 4.9 / 1.3 | `FPSControllerNEW` speeds |
| granny walk / follow | 2.35 / 3.15 | `EnemyAIGranny.walkSpeed` |
| see / hear / attack range | 14.0 / 9.0 / 1.9 | `seeRange`, `attackRange` |
| days per difficulty | 6 / 5 / 5 / 5 | `DifficultyData` |
| granny speed by difficulty | 2.05 / 2.35 / 2.75 / 3.15 | `DifficultyData` |

## Assets

Extracted from `assets/bin/Data` in the base APK:

- `level2` is the playable scene (3,324 transforms, 3,337 GameObjects).
- 359 meshes / 231,969 faces exported to `assets/scene.bin`.
- 713 textures; the 256 most-used diffuse maps form `assets/atlas.png`.
- 235 audio clips extracted to the analysis workspace.

Unity rules that had to be reproduced:

1. `Mesh.m_SubMeshes[i].firstByte` is a **byte** offset into the index buffer.
2. Submesh indices are absolute vertex indices, not relative to `firstVertex`.
3. Objects named `Combined Mesh (root: scene) *` are already baked in world
   space; their Transform must not be re-applied.
4. When a renderer has fewer materials than the mesh has submeshes, Unity reuses
   the **last** material.
5. Held viewmodel meshes are parented to the player rig and are excluded.
6. Helper volumes (`LightBox`, `Invinsible*`, `BlackScreen`) are excluded.
