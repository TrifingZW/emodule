import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

struct LandSeaParams {
    tile_width: f32,
    tile_height: f32,
    size: vec2<f32>,
    grid_size: vec2<u32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var base_color_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var base_color_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var sea_mask_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> params: LandSeaParams;

// 相邻列的中心间距与六边形宽度之比。
const COLUMN_STEP_RATIO: f32 = 0.75;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let pixel: vec2<f32> = params.size * mesh.uv;
    let grid: vec2<i32> = get_grid_coords(pixel);
    let grid_size: vec2<i32> = vec2<i32>(params.grid_size);

    // 地图边界外的区域按海面处理，避免顶部/底部半格露出陆地底色。
    if (grid.x < 0 || grid.y < 0 || grid.x >= grid_size.x || grid.y >= grid_size.y) {
        return textureSample(base_color_texture, base_color_sampler, mesh.uv);
    }

    let mask: vec4<f32> = textureLoad(sea_mask_texture, grid, 0);

    // 遮罩 alpha 非零表示陆地，直接丢弃片元。
    if (mask.a != 0.0) {
        discard;
    }

    return textureSample(base_color_texture, base_color_sampler, mesh.uv);
}

// 将像素坐标映射为六边形网格坐标，与 CPU 端的 world_to_grid 保持一致。
fn get_grid_coords(pixel: vec2<f32>) -> vec2<i32> {
    // 去掉首个网格的中心偏移。
    let local: vec2<f32> = pixel - vec2<f32>(params.tile_width * 0.5, params.tile_height * 0.5);
    let q: f32 = local.x / (params.tile_width * COLUMN_STEP_RATIO);
    let r: f32 = local.y / params.tile_height - q * 0.5;

    let axial: vec2<i32> = round_axial(q, r);
    let row: i32 = axial.y + (axial.x - (axial.x & 1)) / 2;

    return vec2<i32>(axial.x, row);
}

// 将分数轴坐标取整为最近的整数轴坐标。
fn round_axial(q: f32, r: f32) -> vec2<i32> {
    let s: f32 = -q - r;
    var rounded_q: f32 = round(q);
    var rounded_r: f32 = round(r);
    let rounded_s: f32 = round(s);

    let q_delta: f32 = abs(rounded_q - q);
    let r_delta: f32 = abs(rounded_r - r);
    let s_delta: f32 = abs(rounded_s - s);

    if (q_delta > r_delta && q_delta > s_delta) {
        rounded_q = -rounded_r - rounded_s;
    } else if (r_delta > s_delta) {
        rounded_r = -rounded_q - rounded_s;
    }

    return vec2<i32>(i32(rounded_q), i32(rounded_r));
}
