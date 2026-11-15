import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

struct InstanceParams {
    tint: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var base_color_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var base_color_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> params: InstanceParams;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let color: vec4<f32> = textureSample(base_color_texture, base_color_sampler, mesh.uv);

    // 国家配色与图集纹素相乘，保留地形的明暗与细节。
    return color * params.tint;
}
