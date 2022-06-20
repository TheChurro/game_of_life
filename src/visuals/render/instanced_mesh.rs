use crate::bevy::reflect::TypeUuid;
use crate::bevy::render::render_resource::std140::AsStd140;
use bevy::{
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    math::{Mat4, Size, Vec4},
    pbr::{
        GlobalLightMeta, GpuLights, LightMeta, MeshPipelineKey, MeshUniform, MeshViewBindGroup,
        NotShadowCaster, NotShadowReceiver, SetMeshBindGroup, SetShadowViewBindGroup, Shadow,
        ShadowPipeline, StandardMaterial, ViewClusterBindings, ViewShadowBindings,
        CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT,
    },
    prelude::{
        Assets, Commands, Component, ComputedVisibility, Entity, FromWorld, GlobalTransform,
        Handle, HandleUntyped, Image, Local, Mesh, Plugin, Query, Res, Transform, Visibility, With,
        Without, World,
    },
    render::{
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, EntityRenderCommand, RenderCommandResult, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, GpuImage, TextureFormatPixelInfo},
        view::{NoFrustumCulling, ViewUniform, ViewUniforms},
        RenderApp, RenderStage,
    },
    utils::HashMap,
};
use bytemuck::Pod;

#[derive(Default)]
pub struct InstanceMeshRenderPlugin;

pub const INSTANCE_MESH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 16278916168802320000);

impl Plugin for InstanceMeshRenderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut assets = app.world.resource_mut::<Assets<_>>();
        assets.set_untracked(
            INSTANCE_MESH_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("instanced_mesh.wgsl")),
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<InstancedMeshPipeline>()
                .add_system_to_stage(RenderStage::Extract, extract_meshes)
                .add_system_to_stage(RenderStage::Prepare, prepare_instance_buffers)
                .add_system_to_stage(RenderStage::Queue, queue_mesh_view_bind_groups)
                .add_render_command::<Shadow, DrawShadowMesh>();
        }
    }
}

#[derive(Component)]
pub struct MeshInstance {
    pub mesh: Handle<Mesh>,
}

#[derive(Component, Clone, Copy, Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct InstanceTransforms {
    transform_0: Vec4,
    transform_1: Vec4,
    transform_2: Vec4,
    transform_3: Vec4,
    inverse_transpose_transform_0: Vec4,
    inverse_transpose_transform_1: Vec4,
    inverse_transpose_transform_2: Vec4,
    inverse_transpose_transform_3: Vec4,
}

impl InstanceTransforms {
    fn new(transform: Mat4) -> Self {
        let inverse_transpose = transform.inverse().transpose();
        Self {
            transform_0: transform.col(0),
            transform_1: transform.col(1),
            transform_2: transform.col(2),
            transform_3: transform.col(3),
            inverse_transpose_transform_0: inverse_transpose.col(0),
            inverse_transpose_transform_1: inverse_transpose.col(1),
            inverse_transpose_transform_2: inverse_transpose.col(2),
            inverse_transpose_transform_3: inverse_transpose.col(3),
        }
    }
}

#[derive(Component)]
pub(crate) struct InstancedMeshTransforms {
    pub transforms: Vec<InstanceTransforms>,
}

// NOTE: These must match the bit flags in bevy_pbr2/src/render/mesh.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    struct MeshFlags: u32 {
        const SHADOW_RECEIVER            = (1 << 0);
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

pub fn extract_meshes(
    mut commands: Commands,
    mut previous_caster_len: Local<usize>,
    mut previous_not_caster_len: Local<usize>,
    caster_query: Query<
        (
            &ComputedVisibility,
            &GlobalTransform,
            &MeshInstance,
            &Handle<StandardMaterial>,
            Option<&NotShadowReceiver>,
        ),
        Without<NotShadowCaster>,
    >,
    not_caster_query: Query<
        (
            &ComputedVisibility,
            &GlobalTransform,
            &MeshInstance,
            Option<&NotShadowReceiver>,
        ),
        With<NotShadowCaster>,
    >,
) {
    let mut caster_map =
        HashMap::<(Handle<Mesh>, Handle<StandardMaterial>), InstancedMeshTransforms>::with_capacity(
            *previous_caster_len,
        );
    for (computed_visibility, transform, instance, material, _) in caster_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }
        let transform = transform.compute_matrix();
        if let Some(instance_data) =
            caster_map.get_mut(&(instance.mesh.clone_weak(), material.clone_weak()))
        {
            instance_data
                .transforms
                .push(InstanceTransforms::new(transform));
        } else {
            caster_map.insert(
                (instance.mesh.clone_weak(), material.clone_weak()),
                InstancedMeshTransforms {
                    transforms: vec![InstanceTransforms::new(transform)],
                },
            );
        }
    }
    *previous_caster_len = caster_map.len();
    commands.spawn_batch(caster_map.into_iter().map(|((a, b), c)| {
        (
            a,
            b,
            c,
            Transform::default(),
            GlobalTransform::default(),
            Visibility { is_visible: true },
            MeshUniform {
                transform: Transform::default().compute_matrix(),
                inverse_transpose_model: Transform::default().compute_matrix(),
                flags: MeshFlags::SHADOW_RECEIVER.bits(),
            },
            ComputedVisibility { is_visible: true },
            NoFrustumCulling,
        )
    }));

    let mut not_caster_map =
        HashMap::<Handle<Mesh>, InstancedMeshTransforms>::with_capacity(*previous_not_caster_len);
    for (computed_visibility, transform, instance, _) in not_caster_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }
        let transform = transform.compute_matrix();
        if let Some(instance_data) = not_caster_map.get_mut(&instance.mesh) {
            instance_data
                .transforms
                .push(InstanceTransforms::new(transform));
        } else {
            not_caster_map.insert(
                instance.mesh.clone_weak(),
                InstancedMeshTransforms {
                    transforms: vec![InstanceTransforms::new(transform)],
                },
            );
        }
    }
    *previous_not_caster_len = not_caster_map.capacity();
    commands.spawn_batch(
        not_caster_map
            .into_iter()
            .map(|x| (NotShadowCaster, x.0, x.1)),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn queue_mesh_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh_pipeline: Res<InstancedMeshPipeline>,
    shadow_pipeline: Res<ShadowPipeline>,
    light_meta: Res<LightMeta>,
    global_light_meta: Res<GlobalLightMeta>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &ViewShadowBindings, &ViewClusterBindings)>,
) {
    if let (Some(view_binding), Some(light_binding), Some(point_light_binding)) = (
        view_uniforms.uniforms.binding(),
        light_meta.view_gpu_lights.binding(),
        global_light_meta.gpu_point_lights.binding(),
    ) {
        for (entity, view_shadow_bindings, view_cluster_bindings) in views.iter() {
            let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: view_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: light_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(
                            &view_shadow_bindings.point_light_depth_texture_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::Sampler(&shadow_pipeline.point_light_sampler),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(
                            &view_shadow_bindings.directional_light_depth_texture_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::Sampler(
                            &shadow_pipeline.directional_light_sampler,
                        ),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: point_light_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 7,
                        resource: view_cluster_bindings.light_index_lists_binding().unwrap(),
                    },
                    BindGroupEntry {
                        binding: 8,
                        resource: view_cluster_bindings.offsets_and_counts_binding().unwrap(),
                    },
                ],
                label: Some("mesh_view_bind_group"),
                layout: &mesh_pipeline.view_layout,
            });

            commands.entity(entity).insert(MeshViewBindGroup {
                value: view_bind_group,
            });
        }
    }
}

#[derive(Clone)]
pub struct InstancedMeshPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub skinned_mesh_layout: BindGroupLayout,
    // This dummy white texture is to be used in place of optional StandardMaterial textures
    pub dummy_white_gpu_image: GpuImage,
    pub clustered_forward_buffer_binding_type: BufferBindingType,
}

const MAX_JOINTS: usize = 256;
const JOINT_SIZE: usize = std::mem::size_of::<Mat4>();
const JOINT_BUFFER_SIZE: usize = MAX_JOINTS * JOINT_SIZE;

impl FromWorld for InstancedMeshPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let clustered_forward_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);
        let cluster_min_binding_size = match clustered_forward_buffer_binding_type {
            BufferBindingType::Storage { .. } => None,
            BufferBindingType::Uniform => BufferSize::new(16384),
        };
        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
                    },
                    count: None,
                },
                // Lights
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(GpuLights::std140_size_static() as u64),
                    },
                    count: None,
                },
                // Point Shadow Texture Cube Array
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Depth,
                        #[cfg(not(feature = "webgl"))]
                        view_dimension: TextureViewDimension::CubeArray,
                        #[cfg(feature = "webgl")]
                        view_dimension: TextureViewDimension::Cube,
                    },
                    count: None,
                },
                // Point Shadow Texture Array Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Comparison),
                    count: None,
                },
                // Directional Shadow Texture Array
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Depth,
                        #[cfg(not(feature = "webgl"))]
                        view_dimension: TextureViewDimension::D2Array,
                        #[cfg(feature = "webgl")]
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Directional Shadow Texture Array Sampler
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Comparison),
                    count: None,
                },
                // PointLights
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: clustered_forward_buffer_binding_type,
                        has_dynamic_offset: false,
                        // NOTE (when no storage buffers): Static size for uniform buffers.
                        // GpuPointLight has a padded size of 64 bytes, so 16384 / 64 = 256
                        // point lights max
                        min_binding_size: cluster_min_binding_size,
                    },
                    count: None,
                },
                // ClusteredLightIndexLists
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: clustered_forward_buffer_binding_type,
                        has_dynamic_offset: false,
                        // NOTE (when no storage buffers): With 256 point lights max, indices
                        // need 8 bits so use u8
                        min_binding_size: cluster_min_binding_size,
                    },
                    count: None,
                },
                // ClusterOffsetsAndCounts
                BindGroupLayoutEntry {
                    binding: 8,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: clustered_forward_buffer_binding_type,
                        has_dynamic_offset: false,
                        // NOTE (when no storage buffers): The offset needs to address 16384
                        // indices, which needs 14 bits. The count can be at most all 256 lights
                        // so 8 bits.
                        // NOTE: Pack the offset into the upper 19 bits and the count into the
                        // lower 13 bits.
                        min_binding_size: cluster_min_binding_size,
                    },
                    count: None,
                },
            ],
            label: Some("mesh_view_layout"),
        });

        let mesh_binding = BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: BufferSize::new(MeshUniform::std140_size_static() as u64),
            },
            count: None,
        };

        let mesh_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[mesh_binding],
            label: Some("mesh_layout"),
        });

        let skinned_mesh_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(JOINT_BUFFER_SIZE as u64),
                    },
                    count: None,
                }],
                label: Some("skinned_mesh_layout"),
            });

        // A 1x1x1 'all 1.0' texture to use as a dummy texture to use in place of optional StandardMaterial textures
        let dummy_white_gpu_image = {
            let image = Image::new_fill(
                Extent3d::default(),
                TextureDimension::D2,
                &[255u8; 4],
                TextureFormat::bevy_default(),
            );
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = render_device.create_sampler(&image.sampler_descriptor);

            let format_size = image.texture_descriptor.format.pixel_size();
            let render_queue = world.resource_mut::<RenderQueue>();
            render_queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        std::num::NonZeroU32::new(
                            image.texture_descriptor.size.width * format_size as u32,
                        )
                        .unwrap(),
                    ),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );

            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: Size::new(
                    image.texture_descriptor.size.width as f32,
                    image.texture_descriptor.size.height as f32,
                ),
            }
        };
        InstancedMeshPipeline {
            view_layout,
            mesh_layout,
            skinned_mesh_layout,
            clustered_forward_buffer_binding_type,
            dummy_white_gpu_image,
        }
    }
}

impl SpecializedMeshPipeline for InstancedMeshPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut vertex_attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ];

        let mut shader_defs = Vec::new();
        if layout.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push(String::from("VERTEX_TANGENTS"));
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(3));
        }

        // TODO: consider exposing this in shaders in a more generally useful way, such as:
        // # if AVAILABLE_STORAGE_BUFFER_BINDINGS == 3
        // /* use storage buffers here */
        // # elif
        // /* use uniforms here */
        if !matches!(
            self.clustered_forward_buffer_binding_type,
            BufferBindingType::Storage { .. }
        ) {
            shader_defs.push(String::from("NO_STORAGE_BUFFERS_SUPPORT"));
        }

        let mut bind_group_layout = vec![self.view_layout.clone()];
        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            shader_defs.push(String::from("SKINNED"));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
            bind_group_layout.push(self.skinned_mesh_layout.clone());
        } else {
            bind_group_layout.push(self.mesh_layout.clone());
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        let instance_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceTransforms>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 6, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 7, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 2,
                    shader_location: 8, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 3,
                    shader_location: 9, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 4,
                    shader_location: 10, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 5,
                    shader_location: 11, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 6,
                    shader_location: 12, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size() * 7,
                    shader_location: 13, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
            ],
        };

        let (label, blend, depth_write_enabled);
        if key.contains(MeshPipelineKey::TRANSPARENT_MAIN_PASS) {
            label = "transparent_mesh_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // For the transparent pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else {
            label = "opaque_mesh_pipeline".into();
            blend = Some(BlendState::REPLACE);
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
        }

        #[cfg(feature = "webgl")]
        shader_defs.push(String::from("NO_ARRAY_TEXTURES_SUPPORT"));

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: INSTANCE_MESH_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout, instance_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: INSTANCE_MESH_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(bind_group_layout),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some(label),
        })
    }
}

#[derive(Component)]
pub struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &InstancedMeshTransforms)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in query.iter() {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(instance_data.transforms.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.transforms.len(),
        });
    }
}

pub type DrawShadowMesh = (
    SetItemPipeline,
    SetShadowViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawInstancedMesh,
);

pub struct DrawInstancedMesh;
impl EntityRenderCommand for DrawInstancedMesh {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<Handle<Mesh>>>,
        SQuery<Read<InstanceBuffer>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, mesh_query, instanced_buffer_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handle = mesh_query.get(item).unwrap();
        let instance_buffer = instanced_buffer_query.get_inner(item).unwrap();

        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
                }
                GpuBufferInfo::NonIndexed { vertex_count } => {
                    pass.draw(0..*vertex_count, 0..instance_buffer.length as u32);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}
