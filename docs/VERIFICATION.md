# Verification

## 1. Binary/metadata agreement

`global-metadata.dat` v31 was decoded twice by independent means:

| Quantity | REA `inspect_il2cpp_metadata` | independent parser | match |
|---|---|---|---|
| types | 10,037 | 10,037 | ✅ |
| methods | 65,995 | 65,995 | ✅ |
| fields | 40,942 | 40,942 | ✅ |
| parameters | 67,315 | 67,315 | ✅ |
| images | 75 | 75 | ✅ |
| `Assembly-CSharp` types | 885 | 885 | ✅ |
| `EnemyAIGranny` method start | 30260 (+22) | 30260 (+22) | ✅ |
| `AIfollow` method start | 30186 (+13) | 30186 (+13) | ✅ |
| `RotateCamera` method start | 29685 (+3) | 29685 (+3) | ✅ |

## 2. Code identity

Method RVAs resolved from `method_addresses` and disassembled from
`libil2cpp.so` with Capstone (AArch64). Spot checks:

| Method | RVA | First instruction |
|---|---|---|
| `EnemyAIGranny.Start` | `0x1453e28` | `stp d9, d8, [sp, #-0x70]!` |
| `EnemyAIGranny.GrannyDecisions` | `0x145b020` | `stp x30, x21, [sp, #-0x20]!` |
| `AIfollow.Update` | `0x144f1e4` | decoded |
| `FPSControllerNEW.FixedUpdate` | `0x14e4268` | decoded |
| `StartNewDay.Start` | `0x154e1a4` | decoded |
| `OpenCloseDoors.Update` | `0x1500d38` | decoded |

`GrannyDecisions` reads six busy flags at `+0x2c1`, `+0x2c2`, `+0x2c3`,
`+0x2c5`, `+0x2c6`, `+0x2c7` and returns early when any is non-zero. Those six
are `AiBusy` in `src/game.rs`, and `tests/logic.rs::ai_busy_any_covers_all_six_flags`
pins each one.

## 3. Field-name evidence

Every Rust field group is named from `global-metadata.dat`:

- `EnemyAIGranny`: `seeRange`, `attackRange`, `walkSpeed`, `grannysFollowSpeed`,
  `GrannySearching`, `GrannySearch`, `grannyHearPlayer`, `grannySeeDoor`,
  `hidingUnderBed1..3`, `playerHiding`, `droppingBeartrap`, `freeze`
- `FPSControllerNEW`: `forwardSpeed`, `backwardSpeed`, `sidestepSpeed`,
  `jumpSpeed`, `playerCrouch`, `playerCaught`, `fallDead`
- `InventoryController`: `padlockCode`, `armborst`, `pepperspray`, `bearTrap`,
  `carKey`, `carBattery`, `gascan`, `wrench`, `sparkplug`
- `StartNewDay`: `day1..day6`, `daysCounter`, `RandomGrannyPos`,
  `RandomPlaceObjects`, `RandomKeyPlace`, `RandomKofotPlace`
- `OpenCloseDoors`: `doorOpen`, `doorClose`, `doorLocked`, `doorLockedLjud`

## 4. Asset fidelity

| Item | Count | Source |
|---|---|---|
| scene transforms | 3,324 | `level2` |
| meshes exported | 359 | `Mesh.export()` |
| faces | 231,969 | index buffers |
| textures | 713 | `Texture2D` |
| audio clips | 235 | `AudioClip.samples` |
| submesh draws in pack | 2,299 | renderer/mesh join |

Unity rules verified by rendering and corrected in the exporter:

1. `m_SubMeshes[i].firstByte` is a **byte** offset (dividing by 2 produced
   overlapping meshes and 63,000-unit bounding boxes).
2. Submesh indices are absolute, not `firstVertex`-relative.
3. `Combined Mesh (root: scene) *` objects are already world-space.
4. Renderers with fewer materials than submeshes reuse the **last** material.
5. Held viewmodels parented to the player rig must be excluded.
6. Helper volumes (`LightBox`, `Invinsible*`, `BlackScreen`) must be excluded.

## 5. Runtime

Desktop build renders `level2` from the recovered pack on a virtual display
(Xvfb + lavapipe), starting at `PlayerStartPos = (13.75, 21.03, -7.16)`.

`cargo test` — 11 tests covering:

- `GrannyDecisions` busy-flag early-out (all six flags)
- difficulty ordering and day counts
- `StartNewDay` end-of-run
- `OpenCloseDoors` open/close/locked transitions
- inventory dedupe, pickup-once
- crouch vs run speed and noise
- Granny acquiring a visible player
- day timer rollover

## 6. End-to-end on Android

The CI-built APK was installed into an Altaris clone and launched on a
virtual display. Logcat shows the full wgpu init chain:

```
Load .../libgranny_rs.so using class loader ns clns-10: ok
I/wgpu_core::instance: Adapter Vulkan AdapterInfo {
    name: "Adreno (TM) 825", vendor: 20803, device_type: IntegratedGpu,
    driver: "Qualcomm Technologies Inc. Adreno Vulkan Driver",
    backend: Vulkan }
I/granny_rs::android: renderer ready
```

The rendered frame shows the extracted `level2` geometry and texture atlas
inside the clone. See `docs/screenshot.png` for the desktop render and
`docs/android.png` for the on-device capture.

## 7. Motion

The desktop and Android builds share `App::frame`, which drives
`World::tick`. With `GRANNY_AUTOWALK=1` the same tick path runs without
needing keyboard focus; two captures 4 s apart differ on **41.6%** of pixels,
which confirms the render loop, player integration and AI stepping are all
live rather than a single cached frame.

This also caught a real bug: a failed surface acquisition used to
`return` from `RedrawRequested`, ending the winit redraw chain permanently and
freezing the view. Surface errors are now handled (`Lost` reconfigures) and the
next redraw is always requested.
