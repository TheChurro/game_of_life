use bevy::prelude::{Bundle, ComputedVisibility, GlobalTransform, Handle, Transform, Visibility};

use self::{instanced_mesh::MeshInstance, instanced_mesh_material::InstancedSpecializedMaterial};

pub mod instanced_mesh;
pub mod instanced_mesh_material;
pub mod instanced_pbr;
pub mod instanced_shadows;

#[derive(Bundle)]
pub struct InstancedPbrBundle<M: InstancedSpecializedMaterial> {
    pub mesh: MeshInstance,
    pub material: Handle<M>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

impl<M: InstancedSpecializedMaterial> Default for InstancedPbrBundle<M> {
    fn default() -> Self {
        Self {
            mesh: MeshInstance {
                mesh: Default::default(),
            },
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
        }
    }
}
