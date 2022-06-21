#import bevy_pbr::mesh_struct

// NOTE: Keep in sync with pbr.wgsl
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var<uniform> mesh: Mesh;

#ifdef SKINNED
[[group(1), binding(1)]]
var<uniform> joint_matrices: SkinnedMesh;
#import bevy_pbr::skinning
#endif

struct Vertex {
    [[location(0)]] position: vec3<f32>;
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
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
#ifdef SKINNED
    let model = skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    let model = mesh.model;
#endif
    let instance_model = mat4x4<f32>(
        vertex.instance_model_0,
        vertex.instance_model_1,
        vertex.instance_model_2,
        vertex.instance_model_3
    ) * model;

    var out: VertexOutput;
    out.clip_position = view.view_proj * instance_model * vec4<f32>(vertex.position, 1.0);
    return out;
}
