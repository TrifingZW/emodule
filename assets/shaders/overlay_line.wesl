import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

struct OverlayLineStyle {
    color: vec4<f32>,
    dash_length: f32,
    gap_length: f32,
    padding: vec2<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> style: OverlayLineStyle;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // uv.x 是沿线段方向的世界距离；间隔段直接丢弃，形成虚线。
    if (style.dash_length > 0.0) {
        let period: f32 = style.dash_length + style.gap_length;
        if (fract(mesh.uv.x / period) * period > style.dash_length) {
            discard;
        }
    }

    return style.color;
}
