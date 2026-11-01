//! Runtime entity meshes: Granny, items, doors, HUD.
//!
//! The original spawns these as prefabs from `sharedassets*.assets`; the remake
//! builds simple proxies so the recovered AI and item logic stays testable
//! without a full skinned-mesh pipeline.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct V {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub atlas: [f32; 4],
}

pub struct Builder {
    pub verts: Vec<V>,
    pub idx: Vec<u32>,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            verts: Vec::new(),
            idx: Vec::new(),
        }
    }

    fn push_tri(&mut self, a: Vec3, b: Vec3, c: Vec3, col: [f32; 3]) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        let base = self.verts.len() as u32;
        for p in [a, b, c] {
            self.verts.push(V {
                pos: p.to_array(),
                normal: n.to_array(),
                uv: [0.0, 0.0],
                atlas: [0.0, 0.0, 1.0, 255.0],
            });
        }
        self.idx.extend_from_slice(&[base, base + 1, base + 2]);
        let _ = col;
    }

    pub fn box_at(&mut self, center: Vec3, half: Vec3, rot: f32) {
        let q = Quat::from_rotation_y(rot);
        let m = Mat4::from_scale_rotation_translation(Vec3::ONE, q, center);
        let c = [
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ]
        .map(|v| m.transform_point3(v * half));
        let f = [
            [0, 1, 2, 3],
            [5, 4, 7, 6],
            [4, 0, 3, 7],
            [1, 5, 6, 2],
            [3, 2, 6, 7],
            [4, 5, 1, 0],
        ];
        for face in f {
            self.push_tri(c[face[0]], c[face[1]], c[face[2]], [1.0, 1.0, 1.0]);
            self.push_tri(c[face[0]], c[face[2]], c[face[3]], [1.0, 1.0, 1.0]);
        }
    }

    /// A capsule-ish body: stacked boxes, which is enough to read as a figure.
    pub fn figure(&mut self, pos: Vec3, yaw: f32, height: f32) {
        let s = height / 1.75;
        // legs
        self.box_at(
            pos + Vec3::new(-0.16 * s, 0.42 * s, 0.0),
            Vec3::new(0.11 * s, 0.42 * s, 0.11 * s),
            yaw,
        );
        self.box_at(
            pos + Vec3::new(0.16 * s, 0.42 * s, 0.0),
            Vec3::new(0.11 * s, 0.42 * s, 0.11 * s),
            yaw,
        );
        // torso
        self.box_at(
            pos + Vec3::new(0.0, 1.05 * s, 0.0),
            Vec3::new(0.26 * s, 0.34 * s, 0.17 * s),
            yaw,
        );
        // arms
        self.box_at(
            pos + Vec3::new(-0.36 * s, 1.05 * s, 0.0),
            Vec3::new(0.09 * s, 0.3 * s, 0.09 * s),
            yaw,
        );
        self.box_at(
            pos + Vec3::new(0.36 * s, 1.05 * s, 0.0),
            Vec3::new(0.09 * s, 0.3 * s, 0.09 * s),
            yaw,
        );
        // head
        self.box_at(
            pos + Vec3::new(0.0, 1.55 * s, 0.0),
            Vec3::new(0.17 * s, 0.2 * s, 0.17 * s),
            yaw,
        );
    }
}
