use crate::entity::{Component, ComponentID};
#[derive(ComponentId)]
pub struct TerrainComponent {
    pub shader_program: usize,
}
