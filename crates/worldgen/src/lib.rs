#[derive(Clone, Copy)]
pub struct GenParams {
    pub seed: u64,
    pub chunk_size: u32,
    pub scale: f32,
    pub height: f32,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkKey {
    pub x: i32,
    pub z: i32,
}
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}
pub struct WorldGen {
    p: GenParams,
}
impl WorldGen {
    pub fn new(p: GenParams) -> Self {
        Self { p }
    }
    pub fn chunk_mesh(&self, key: ChunkKey) -> MeshData {
        let n = self.p.chunk_size as usize;
        let stride = self.p.scale * (self.p.chunk_size - 1) as f32;
        let base_x = key.x as f32 * stride;
        let base_z = key.z as f32 * stride;
        let mut positions = Vec::with_capacity(n * n);
        let mut uvs = Vec::with_capacity(n * n);
        let mut indices = Vec::with_capacity((n - 1) * (n - 1) * 6);
        for j in 0..n {
            for i in 0..n {
                let wx = base_x + (i as f32) * self.p.scale;
                let wz = base_z + (j as f32) * self.p.scale;
                let y = (wx * 0.0113).sin() * 0.6 + (wz * 0.0097).cos() * 0.4;
                positions.push([wx, y * self.p.height, wz]);
                uvs.push([i as f32 / (n - 1) as f32, j as f32 / (n - 1) as f32]);
            }
        }
        for j in 0..(n - 1) {
            for i in 0..(n - 1) {
                let a = (j * n + i) as u32;
                let b = a + 1;
                let c = (j * n + i + n) as u32;
                let d = c + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
        MeshData {
            positions,
            normals,
            uvs,
            indices,
        }
    }
}
