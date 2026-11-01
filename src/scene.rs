//! Loader for the recovered Unity scene pack (`scene.bin`).

use glam::{Mat4, Vec3};

pub struct Draw {
    pub name: String,
    pub model: Mat4,
    pub idx_off: u32,
    pub idx_count: u32,
    pub slot: u32,
    pub active: bool,
}

pub struct Scene {
    pub verts: Vec<Vec3>,
    /// Raw Unity UVs (may exceed 0..1 for tiling).
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub draws: Vec<Draw>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

fn rd_u32(b: &[u8], o: &mut usize) -> u32 {
    let v = u32::from_le_bytes(b[*o..*o + 4].try_into().unwrap());
    *o += 4;
    v
}

impl Scene {
    pub fn load(bytes: &[u8]) -> anyhow::Result<Scene> {
        anyhow::ensure!(bytes.len() > 16 && &bytes[0..4] == b"GRS4", "bad magic");
        let mut o = 4usize;
        let nv = rd_u32(bytes, &mut o) as usize;
        let ni = rd_u32(bytes, &mut o) as usize;

        let mut verts = Vec::with_capacity(nv);
        let mut uvs = Vec::with_capacity(nv);
        for _ in 0..nv {
            let x = f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
            let y = f32::from_le_bytes(bytes[o + 4..o + 8].try_into().unwrap());
            let z = f32::from_le_bytes(bytes[o + 8..o + 12].try_into().unwrap());
            let u = f32::from_le_bytes(bytes[o + 12..o + 16].try_into().unwrap());
            let v = f32::from_le_bytes(bytes[o + 16..o + 20].try_into().unwrap());
            o += 20;
            verts.push(Vec3::new(x, y, z));
            uvs.push([u, v]);
        }

        let mut indices = Vec::with_capacity(ni);
        for _ in 0..ni {
            indices.push(rd_u32(bytes, &mut o));
        }

        let nd = rd_u32(bytes, &mut o) as usize;
        let mut draws = Vec::with_capacity(nd);
        for _ in 0..nd {
            let nlen = bytes[o] as usize;
            o += 1;
            let name = String::from_utf8_lossy(&bytes[o..o + nlen]).to_string();
            o += nlen;
            let mut m = [0f32; 16];
            for v in m.iter_mut() {
                *v = f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
                o += 4;
            }
            // Packed row-major (row i, col j at m[i*4+j]); glam wants columns.
            let model = Mat4::from_cols_array(&m).transpose();
            let idx_off = rd_u32(bytes, &mut o);
            let idx_count = rd_u32(bytes, &mut o);
            let slot = rd_u32(bytes, &mut o);
            let active = bytes[o] != 0;
            o += 1;
            draws.push(Draw {
                name,
                model,
                idx_off,
                idx_count,
                slot,
                active,
            });
        }

        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in &verts {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }

        Ok(Scene {
            verts,
            uvs,
            indices,
            draws,
            bounds_min: lo,
            bounds_max: hi,
        })
    }

    /// Flatten the visible scene into world-space triangles for collision.
    pub fn collider_tris(&self, limit: usize) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        for d in &self.draws {
            if !d.active {
                continue;
            }
            let s = d.idx_off as usize;
            let e = (d.idx_off + d.idx_count) as usize;
            let mut k = s;
            while k + 2 < e && out.len() < limit {
                let a = self.verts[self.indices[k] as usize];
                let b = self.verts[self.indices[k + 1] as usize];
                let c = self.verts[self.indices[k + 2] as usize];
                out.push([
                    d.model.transform_point3(a),
                    d.model.transform_point3(b),
                    d.model.transform_point3(c),
                ]);
                k += 3;
            }
        }
        out
    }
}
