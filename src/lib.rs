mod error;
pub use error::AyudeError;

pub mod graphics;

pub mod catalog;
pub use catalog::Catalog;
use smallvec::SmallVec;

pub mod import_gltf;

#[derive(Debug)]
pub struct Scene {
    pub nodes: Vec<Node>,
    pub root_nodes: SmallVec<[u16; 4]>,
    pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub parent: Option<u16>,
    pub children: SmallVec<[u16; 4]>,
    pub transform: Transform,
    pub meshes: Vec<graphics::Mesh>,
    pub skin: Option<Skin>,
}

#[derive(Debug, Clone)]
pub struct Skin {
    pub joints: SmallVec<[u16; 4]>,
    pub skeleton: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct Transform([[f32; 4]; 4]);

impl Transform {
    pub fn new(mat: [[f32; 4]; 4]) -> Transform {
        Transform(mat)
    }
    pub fn mat4(&self) -> &[[f32; 4]; 4] {
        &self.0
    }
}