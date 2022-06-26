// use bevy::{prelude::{Res, Query, Handle, Mesh, Without, With, ResMut, error}, render::{render_phase::{DrawFunctions, RenderPhase}, render_asset::RenderAssets, render_resource::{SpecializedMeshPipelines, PipelineCache}, view::VisibleEntities}, pbr::{Shadow, ShadowPipeline, NotShadowCaster, ViewLightEntities, LightEntity, CubemapVisibleEntities, ExtractedPointLight, ExtractedDirectionalLight, ShadowPipelineKey}};

// use super::instanced_mesh::{InstancedMeshTransforms, DrawShadowMesh};

// #[allow(clippy::too_many_arguments)]
// fn queue_shadows(
//     shadow_draw_functions: Res<DrawFunctions<Shadow>>,
//     shadow_pipeline: Res<ShadowPipeline>,
//     casting_meshes: Query<&Handle<Mesh>, (With<InstancedMeshTransforms>, Without<NotShadowCaster>)>,
//     render_meshes: Res<RenderAssets<Mesh>>,
//     mut pipelines: ResMut<SpecializedMeshPipelines<ShadowPipeline>>,
//     mut pipeline_cache: ResMut<PipelineCache>,
//     view_lights: Query<&ViewLightEntities>,
//     mut view_light_shadow_phases: Query<(&LightEntity, &mut RenderPhase<Shadow>)>,
//     point_light_entities: Query<&CubemapVisibleEntities, With<ExtractedPointLight>>,
//     directional_light_entities: Query<&VisibleEntities, With<ExtractedDirectionalLight>>,
// ) {
//     for view_lights in view_lights.iter() {
//         let draw_shadow_mesh = shadow_draw_functions
//             .read()
//             .get_id::<DrawShadowMesh>()
//             .unwrap();
//         for view_light_entity in view_lights.lights.iter().copied() {
//             let (light_entity, mut shadow_phase) =
//                 view_light_shadow_phases.get_mut(view_light_entity).unwrap();
//             let visible_entities = match light_entity {
//                 LightEntity::Directional { light_entity } => directional_light_entities
//                     .get(*light_entity)
//                     .expect("Failed to get directional light visible entities"),
//                 LightEntity::Point {
//                     light_entity,
//                     face_index,
//                 } => point_light_entities
//                     .get(*light_entity)
//                     .expect("Failed to get point light visible entities")
//                     .get(*face_index),
//             };
//             // NOTE: Lights with shadow mapping disabled will have no visible entities
//             // so no meshes will be queued
//             for entity in visible_entities.iter().copied() {
//                 if let Ok(mesh_handle) = casting_meshes.get(entity) {
//                     if let Some(mesh) = render_meshes.get(mesh_handle) {
//                         let key =
//                             ShadowPipelineKey::from_primitive_topology(mesh.primitive_topology);
//                         let pipeline_id = pipelines.specialize(
//                             &mut pipeline_cache,
//                             &shadow_pipeline,
//                             key,
//                             &mesh.layout,
//                         );

//                         let pipeline_id = match pipeline_id {
//                             Ok(id) => id,
//                             Err(err) => {
//                                 error!("{}", err);
//                                 continue;
//                             }
//                         };

//                         shadow_phase.add(Shadow {
//                             draw_function: draw_shadow_mesh,
//                             pipeline: pipeline_id,
//                             entity,
//                             distance: 0.0, // TODO: sort back-to-front
//                         });
//                     }
//                 }
//             }
//         }
//     }
// }
