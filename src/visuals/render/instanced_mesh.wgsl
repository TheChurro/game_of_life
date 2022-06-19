/// Modified mesh shader to support instanced meshes and skinned meshes in shadow pass

#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] tangent: vec4<f32>;
#endif
#ifdef SKINNED
    [[location(4)]] joint_indices: vec4<u32>;
    [[location(5)]] joint_weights: vec4<f32>;
#endif
    // This following is the mesh information for this instance
    [[location(6)]] instance_model_0: vec4<f32>;
    [[location(7)]] instance_model_1: vec4<f32>;
    [[location(8)]] instance_model_2: vec4<f32>;
    [[location(9)]] instance_model_3: vec4<f32>;
    [[location(10)]] instance_inverse_transpose_model_0: vec4<f32>;
    [[location(11)]] instance_inverse_transpose_model_1: vec4<f32>;
    [[location(12)]] instance_inverse_transpose_model_2: vec4<f32>;
    [[location(13)]] instance_inverse_transpose_model_3: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
};

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;
#ifdef SKINNED
[[group(2), binding(1)]]
var<uniform> joint_matrices: SkinnedMesh;
#import bevy_pbr::skinning
#endif

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
#ifdef SKINNED
    var model = skin_model(vertex.joint_indices, vertex.joint_weights);
    out.world_position = model * vec4<f32>(vertex.position, 1.0);
    out.world_normal = skin_normals(model, vertex.normal);
#ifdef VERTEX_TANGENTS
    out.world_tangent = skin_tangents(model, vertex.tangent);
#endif
#else
    var instance_model = mat4x4<f32>(
        vertex.instance_model_0,
        vertex.instance_model_1,
        vertex.instance_model_2,
        vertex.instance_model_3
    );
    out.world_position = instance_model * vec4<f32>(vertex.position, 1.0);
    out.world_normal = mat3x3<f32>(
        vertex.instance_inverse_transpose_model_0.xyz,
        vertex.instance_inverse_transpose_model_1.xyz,
        vertex.instance_inverse_transpose_model_2.xyz
    ) * vertex.normal;
#ifdef VERTEX_TANGENTS
    out.world_tangent = vec4<f32>(
        mat3x3<f32>(
            vertex.instance_model_0.xyz,
            vertex.instance_model_1.xyz,
            vertex.instance_model_2.xyz
        ) * vertex.tangent.xyz,
        vertex.tangent.w
    );
#endif
#endif

    out.uv = vertex.uv;
    out.clip_position = view.view_proj * out.world_position;
    return out;
}

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}