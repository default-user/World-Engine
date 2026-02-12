/// WGSL shader for rendering the grid floor and instanced cubes.
pub const WORLD_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct InstanceInput {
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_0,
        instance.model_1,
        instance.model_2,
        instance.model_3,
    );
    let world_pos = model * vec4<f32>(vertex.position, 1.0);
    let world_normal = (model * vec4<f32>(vertex.normal, 0.0)).xyz;

    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * world_pos;
    out.world_normal = normalize(world_normal);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.3, 1.0, 0.5));
    let ambient = 0.3;
    let diffuse = max(dot(in.world_normal, light_dir), 0.0);
    let lighting = ambient + diffuse * 0.7;
    return vec4<f32>(in.color.rgb * lighting, in.color.a);
}
"#;

/// WGSL shader for the grid floor.
pub const GRID_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct GridVertex {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct GridOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_grid(vertex: GridVertex) -> GridOutput {
    var out: GridOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(vertex.position, 1.0);
    out.color = vertex.color;
    return out;
}

@fragment
fn fs_grid(in: GridOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
