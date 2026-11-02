//! Shared game application: owns the renderer, world and input state.
//!
//! `main.rs` drives this on desktop via winit; `android.rs` drives the same
//! code through `android-activity`'s `NativeActivity`.

use crate::entities;
use crate::game::{self, Difficulty, World};
use crate::render::{Renderer, Vertex};
use crate::scene;
use glam::Vec3;

#[derive(Default)]
pub struct Keys {
    pub turn_left: bool,
    pub turn_right: bool,
    pub fwd: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub run: bool,
    pub crouch: bool,
}

pub struct App {
    pub renderer: Renderer,
    pub world: World,
    pub keys: Keys,
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    pub last: std::time::Instant,
    /// Deterministic self-test drive. Set by `GRANNY_AUTOWALK`.
    pub autowalk: bool,
}

/// Player spawn, taken from `PlayerStartPos` in the real scene.
pub const SPAWN: Vec3 = Vec3::new(13.75, 21.03, -7.16);
/// `PlayerStartPos`/`Main Camera` face -X, which is yaw -90 degrees in this
/// engine's `forward = (sin yaw, 0, cos yaw)` convention.
pub const SPAWN_YAW: f32 = -std::f32::consts::FRAC_PI_2;
/// `GrannyStartPos1` in the real scene.
pub const GRANNY_SPAWN: Vec3 = Vec3::new(0.53, 7.31, 0.67);

impl App {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        difficulty: Difficulty,
    ) -> anyhow::Result<Self> {
        let mut renderer = Renderer::new(device, queue, config);

        if let Ok(bytes) = crate::assets::load("atlas.png") {
            if let Ok(img) = image::load_from_memory(&bytes) {
                renderer.set_atlas(&img.to_rgba8());
            }
        }

        let bytes = crate::assets::load("scene.bin")?;
        let scene = scene::Scene::load(&bytes)?;

        let mut verts: Vec<Vertex> = Vec::new();
        let mut idx: Vec<u32> = Vec::new();
        for d in &scene.draws {
            if !d.active {
                continue;
            }
            let base = verts.len() as u32;
            let slot = d.slot;
            let (ax, ay, asz) = if slot < 255 {
                (
                    (slot % 16) as f32 / 16.0,
                    (slot / 16) as f32 / 16.0,
                    1.0 / 16.0,
                )
            } else {
                (0.0, 0.0, 1.0)
            };
            let s = d.idx_off as usize;
            let e = (d.idx_off + d.idx_count) as usize;
            let mut k = s;
            while k + 2 < e {
                let (ia, ib, ic) = (
                    scene.indices[k] as usize,
                    scene.indices[k + 1] as usize,
                    scene.indices[k + 2] as usize,
                );
                let (a, b, c) = (
                    d.model.transform_point3(scene.verts[ia]),
                    d.model.transform_point3(scene.verts[ib]),
                    d.model.transform_point3(scene.verts[ic]),
                );
                let n = (b - a).cross(c - a).normalize_or_zero();
                for (p, vi) in [(a, ia), (b, ib), (c, ic)] {
                    verts.push(Vertex {
                        pos: p.to_array(),
                        normal: n.to_array(),
                        uv: scene.uvs[vi],
                        atlas: [ax, ay, asz, slot as f32],
                    });
                }
                let n0 = verts.len() as u32;
                idx.extend_from_slice(&[base + n0 - 3, base + n0 - 2, base + n0 - 1]);
                k += 3;
            }
        }
        renderer.upload(&verts, &idx);

        let mut world = World::new(difficulty, SPAWN);
        world.player.yaw = SPAWN_YAW;
        world.granny.pos = GRANNY_SPAWN;
        world.notify("Day 1");
        // Recovered spawns; the original randomises these per day via
        // RandomKeyPlace / RandomKofotPlace.
        world.pickups.push(game::Pickup {
            pos: Vec3::new(14.4, 22.4, -8.6),
            item: game::Item::Key,
            taken: false,
        });
        world.pickups.push(game::Pickup {
            pos: Vec3::new(-3.5, 1.3, 2.0),
            item: game::Item::Hammer,
            taken: false,
        });
        world.pickups.push(game::Pickup {
            pos: Vec3::new(8.0, 15.4, -39.0),
            item: game::Item::BirdSeed,
            taken: false,
        });
        for d in [
            Vec3::new(11.0, 21.0, -12.0),
            Vec3::new(4.0, 1.0, 0.0),
            Vec3::new(-1.0, 7.0, 0.4),
        ] {
            world.doors.push(game::Door::new(d));
        }

        Ok(App {
            renderer,
            world,
            keys: Keys::default(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            last: std::time::Instant::now(),
            autowalk: std::env::var("GRANNY_AUTOWALK").is_ok(),
        })
    }

    pub fn frame(&mut self, view: &wgpu::TextureView, width: u32, height: u32) {
        let now = std::time::Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.05);
        self.last = now;

        let mut want = glam::Vec2::ZERO;
        if self.keys.fwd {
            want.y += 1.0
        }
        if self.keys.back {
            want.y -= 1.0
        }
        if self.keys.left {
            want.x -= 1.0
        }
        if self.keys.right {
            want.x += 1.0
        }
        self.world.player.crouch = self.keys.crouch;
        if self.autowalk {
            // Drive the same tick path without needing keyboard focus, so CI and
            // headless captures can prove the loop actually advances.
            want.y += 1.0;
            self.world.player.yaw += 0.4 * dt;
        }
        if self.keys.turn_left {
            self.world.player.yaw += 2.2 * dt;
        }
        if self.keys.turn_right {
            self.world.player.yaw -= 2.2 * dt;
        }
        self.world.player.yaw += self.mouse_dx * 0.0022;
        self.world.player.pitch =
            (self.world.player.pitch - self.mouse_dy * 0.0022).clamp(-1.45, 1.45);
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        self.world.tick(dt, want, self.keys.run);

        let mut b = entities::Builder::new();
        b.figure(self.world.granny.pos, 0.0, 1.62);
        for p in &self.world.pickups {
            if p.taken {
                continue;
            }
            b.box_at(
                p.pos + Vec3::new(0.0, 0.18, 0.0),
                Vec3::new(0.14, 0.14, 0.14),
                0.6,
            );
        }
        for d in &self.world.doors {
            if !matches!(d.state, game::DoorState::Open) {
                b.box_at(
                    d.pos + Vec3::new(0.0, 1.05, 0.0),
                    Vec3::new(0.52, 1.05, 0.07),
                    0.0,
                );
            }
        }
        let ev: Vec<Vertex> = b
            .verts
            .iter()
            .map(|v| Vertex {
                pos: v.pos,
                normal: v.normal,
                uv: v.uv,
                atlas: v.atlas,
            })
            .collect();
        self.renderer.upload_entities(&ev, &b.idx);

        let eye = self.world.player.pos + Vec3::new(0.0, 1.55, 0.0);
        let aspect = width as f32 / height.max(1) as f32;
        self.renderer.render(
            view,
            eye,
            self.world.player.yaw,
            self.world.player.pitch,
            aspect,
        );
    }
}
