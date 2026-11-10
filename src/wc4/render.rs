//! WC4 六边形地图渲染。

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    image::Image,
    mesh::{Indices, Mesh, Mesh2d, MeshTag, PrimitiveTopology},
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat},
    shader::ShaderRef,
    sprite::SpriteImageMode,
    sprite_render::{AlphaMode2d, Material2d, MeshMaterial2d},
};

use crate::{
    camera::MapCamera,
    wc4::{
        bin::Bin,
        btl::Btl,
        btl_data::City,
        config::{TerrainXml, Wc4Config},
        grid::{
            GRID_HEIGHT, GRID_WIDTH, grid_to_world, grid_world_size, index_to_grid, world_to_grid,
        },
        resource::{ImageInfo, ImageXml, Wc4Atlas, Wc4Resources},
    },
};

/// 陆地底图的深度。
const Z_LAND_BASE: f32 = -10.0;
/// 海洋底图的深度。
const Z_SEA_BASE: f32 = -9.0;
/// 城市精灵的深度。
const Z_CITY: f32 = 5.0;
/// 地块指示图标的深度。
const Z_TILE_CURSOR: f32 = 11.0;
/// 相邻行的深度间隔，保证行优先的稳定排序。
const Z_ROW_BIAS: f32 = 0.001;
/// 相邻列的深度间隔，用于解决同一行内的排序。
const Z_COLUMN_BIAS: f32 = 0.000_001;
/// 地形层（基础 > 装饰 A > 装饰 B）的相对深度。
const Z_TILE_LAYERS: [f32; 3] = [3.0, 2.0, 1.0];
/// 城市名称与城市精灵的距离。
const CITY_LABEL_OFFSET: f32 = 50.0;
/// 城市名称的字号（逻辑像素）。
const CITY_LABEL_FONT_SIZE: f32 = 16.0;

/// 地形图集着色器路径。
const INSTANCE_SHADER_PATH: &str = "shaders/automatic_instancing.wesl";
/// 海洋遮罩着色器路径。
const SEA_SHADER_PATH: &str = "shaders/land_sea.wesl";

/// 标记 WC4 地图已经生成。
#[derive(Resource)]
pub(crate) struct Wc4MapSpawned;

/// 标记城市名称文本。
#[derive(Component)]
pub(crate) struct CityName;

/// 标记跟随鼠标的地块指示图标。
#[derive(Component)]
pub(crate) struct TileCursor;

/// 单个地形网格实体所需的组件。
type TileBundle = (Mesh2d, MeshMaterial2d<InstanceMaterial>, MeshTag, Transform);

/// 一层地形的网格批次。
type TileBatch = Vec<TileBundle>;

/// 准备完成的地形图集。
struct PreparedAtlas {
    /// 图集纹理。
    image: Handle<Image>,
    /// 地形类型与变体编号到图片帧的查找表。
    frames: HashMap<(i32, i32), ImageFrame>,
    /// 图集纹理尺寸。
    texture_size: Vec2,
}

/// 图集中单张图片的像素区域与锚点。
#[derive(Clone, Copy)]
struct ImageFrame {
    /// 左上角 X。
    x: f32,
    /// 左上角 Y。
    y: f32,
    /// 宽度。
    width: f32,
    /// 高度。
    height: f32,
    /// 锚点 X。
    anchor_x: f32,
    /// 锚点 Y。
    anchor_y: f32,
}

impl ImageFrame {
    /// 从图集配置构造图片帧。
    fn from_info(info: &ImageInfo) -> Self {
        Self {
            x: info.x as f32,
            y: info.y as f32,
            width: info.w as f32,
            height: info.h as f32,
            anchor_x: info.refx as f32,
            anchor_y: info.refy as f32,
        }
    }

    /// 返回图片相对于所在网格中心的偏移。
    fn center_offset(self) -> Vec2 {
        Vec2::new(self.width / 2.0 - self.anchor_x, -self.height / 2.0 + self.anchor_y)
    }

    /// 返回图片在纹理中的区域。
    fn rect(self) -> Rect {
        Rect::new(self.x, self.y, self.x + self.width, self.y + self.height)
    }

    /// 返回图片在纹理中的 UV 范围。
    fn uv(self, texture_size: Vec2) -> [Vec2; 2] {
        [
            Vec2::new(self.x, self.y) / texture_size,
            Vec2::new(self.x + self.width, self.y + self.height) / texture_size,
        ]
    }
}

/// 地图渲染所需的资源集合。
#[derive(SystemParam)]
pub(crate) struct MapAssets<'w> {
    /// 网格资源。
    meshes: ResMut<'w, Assets<Mesh>>,
    /// 纹理资源。
    images: ResMut<'w, Assets<Image>>,
    /// 地形材质资源。
    instance_materials: ResMut<'w, Assets<InstanceMaterial>>,
    /// 海洋材质资源。
    sea_materials: ResMut<'w, Assets<SeaMaterial>>,
    /// 图集配置资源。
    image_xml: Res<'w, Assets<ImageXml>>,
    /// 地形配置资源。
    terrain_xml: Res<'w, Assets<TerrainXml>>,
}

/// 地形层渲染所需的只读数据。
struct TileContext<'a> {
    /// 场景数据。
    btl: &'a Btl,
    /// 世界地形数据。
    bin: &'a Bin,
    /// 地图配置。
    config: &'a Wc4Config,
    /// 图集配置资源。
    image_xml: &'a Assets<ImageXml>,
    /// 地形配置资源。
    terrain_xml: &'a Assets<TerrainXml>,
    /// 纹理资源。
    images: &'a Assets<Image>,
}

/// 生成 WC4 地图实体。
///
/// 所有依赖的资源就绪前不会生成任何实体，避免重复生成。
pub(crate) fn spawn_wc4_map(
    mut commands: Commands,
    bin: Option<Res<Bin>>,
    btl: Option<Res<Btl>>,
    config: Res<Wc4Config>,
    resources: Res<Wc4Resources>,
    mut assets: MapAssets,
) {
    let (Some(bin), Some(btl)) = (bin, btl) else {
        return;
    };

    let context = TileContext {
        btl: &btl,
        bin: &bin,
        config: &config,
        image_xml: &assets.image_xml,
        terrain_xml: &assets.terrain_xml,
        images: &assets.images,
    };

    // 1) 两层地形图集必须同时就绪，否则留到下一帧整体重试。
    let Some(terrain_atlas) = prepare_atlas(&context, &resources.terrain) else {
        return;
    };
    let Some(plant_atlas) = prepare_atlas(&context, &resources.plant) else {
        return;
    };

    // 城市图集未就绪时同样等待，避免生成缺少城市的半成品地图。
    if assets.image_xml.get(&resources.buildings.image_xml).is_none() {
        return;
    }

    // 2) 地形、植被与底图。
    let terrain_batch = tile_batch(
        &context,
        &terrain_atlas,
        assets.instance_materials.add(InstanceMaterial {
            image: terrain_atlas.image.clone(),
        }),
        &mut assets.meshes,
    );
    let plant_batch = tile_batch(
        &context,
        &plant_atlas,
        assets.instance_materials.add(InstanceMaterial {
            image: plant_atlas.image.clone(),
        }),
        &mut assets.meshes,
    );
    let tile_count = terrain_batch.len() + plant_batch.len();

    commands.spawn_batch(terrain_batch);
    commands.spawn_batch(plant_batch);
    spawn_land_base(&mut commands, &btl, &resources);

    let sea_mask = build_sea_mask(&btl, &bin, &mut assets.images);
    if let Some(sea_mask) = sea_mask {
        spawn_sea_base(
            &mut commands,
            &btl,
            &resources,
            sea_mask,
            &mut assets.meshes,
            &mut assets.sea_materials,
        );
    } else {
        warn!("WC4 海洋遮罩尺寸无效，已跳过海洋层");
    }

    // 3) 城市与交互标记。
    spawn_cities(&mut commands, &btl, &bin, &config, &resources, &assets.image_xml);
    commands.spawn((
        TileCursor,
        Sprite::from_image(resources.cursor.clone()),
        Transform::from_xyz(0.0, 0.0, Z_TILE_CURSOR),
    ));

    info!(
        "WC4 地图已生成: {}×{}，{} 个地形网格，{} 个城市",
        btl.master.map_width,
        btl.master.map_height,
        tile_count,
        btl.city_vec.len()
    );

    commands.insert_resource(Wc4MapSpawned);
}

/// 准备单层地形的图集数据。
///
/// 任一依赖资源尚未加载时返回 `None`。
fn prepare_atlas(context: &TileContext, atlas: &Wc4Atlas) -> Option<PreparedAtlas> {
    let image_doc = context.image_xml.get(&atlas.image_xml)?;
    let terrain_doc = context.terrain_xml.get(&context.config.terrain_xml)?;
    let texture = context.images.get(&atlas.image)?;
    let texture_size = Vec2::new(texture.width() as f32, texture.height() as f32);

    if texture_size.min_element() <= 0.0 {
        return None;
    }

    Some(PreparedAtlas {
        image: atlas.image.clone(),
        frames: terrain_frames(terrain_doc, &image_frames(image_doc)),
        texture_size,
    })
}

/// 构建图片名称到图片帧的查找表。
fn image_frames(image_xml: &ImageXml) -> HashMap<&str, ImageFrame> {
    image_xml
        .image_infos
        .iter()
        .map(|info| (info.name.as_str(), ImageFrame::from_info(info)))
        .collect()
}

/// 构建地形类型与变体编号到图片帧的查找表。
fn terrain_frames(
    terrain_xml: &TerrainXml,
    images: &HashMap<&str, ImageFrame>,
) -> HashMap<(i32, i32), ImageFrame> {
    terrain_xml
        .terrains
        .iter()
        .flat_map(|terrain| {
            terrain.tiles.iter().filter_map(|tile| {
                images.get(tile.image.as_str()).map(|&frame| ((terrain.terrain, tile.idx), frame))
            })
        })
        .collect()
}

/// 生成地形网格批次。
fn tile_batch(
    context: &TileContext,
    atlas: &PreparedAtlas,
    material: Handle<InstanceMaterial>,
    meshes: &mut Assets<Mesh>,
) -> TileBatch {
    let frames = &atlas.frames;
    let texture_size = atlas.texture_size;
    let mut batch = Vec::new();
    let master = &context.btl.master;

    for row in 0..master.map_height {
        for column in 0..master.map_width {
            let position = grid_to_world(column, row);

            let Some(x) = master.map_capture_x.checked_add(column) else {
                continue;
            };
            let Some(y) = master.map_capture_y.checked_add(row) else {
                continue;
            };
            let Some(index) = context.bin.index(x, y) else {
                continue;
            };
            let Some(topography) = context.bin.topographies.get(index) else {
                continue;
            };

            let row_bias = y as f32 * Z_ROW_BIAS;
            let column_bias = x as f32 * Z_COLUMN_BIAS;

            // 依次匹配基础层、装饰 A 与装饰 B，顺序与 Z_TILE_LAYERS 一致。
            for (layer, (kind, id, offset)) in [
                (
                    i32::from(topography.r#type),
                    i32::from(topography.id),
                    Vec2::new(f32::from(topography.x), f32::from(topography.y)),
                ),
                (
                    i32::from(topography.decoration_a),
                    i32::from(topography.decoration_a_id),
                    Vec2::new(
                        f32::from(topography.decoration_a_x),
                        f32::from(topography.decoration_a_y),
                    ),
                ),
                (
                    i32::from(topography.decoration_b),
                    i32::from(topography.decoration_b_id),
                    Vec2::new(
                        f32::from(topography.decoration_b_x),
                        f32::from(topography.decoration_b_y),
                    ),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let Some(frame) = frames.get(&(kind, id)) else {
                    continue;
                };
                let frame_offset = frame.center_offset() + Vec2::new(offset.x, -offset.y);

                batch.push((
                    Mesh2d(meshes.add(quad_mesh(*frame, texture_size))),
                    MeshMaterial2d(material.clone()),
                    MeshTag::new(index as u32),
                    Transform::from_xyz(
                        position.x + frame_offset.x,
                        position.y + frame_offset.y,
                        row_bias + column_bias + Z_TILE_LAYERS[layer],
                    ),
                ));
            }
        }
    }

    batch
}

/// 创建带 UV 区域的四边形网格。
fn quad_mesh(frame: ImageFrame, texture_size: Vec2) -> Mesh {
    let half_width = frame.width / 2.0;
    let half_height = frame.height / 2.0;
    let [uv_min, uv_max] = frame.uv(texture_size);

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-half_width, half_height, 0.0],
                [half_width, half_height, 0.0],
                [half_width, -half_height, 0.0],
                [-half_width, -half_height, 0.0],
            ],
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            vec![
                [uv_min.x, uv_min.y],
                [uv_max.x, uv_min.y],
                [uv_max.x, uv_max.y],
                [uv_min.x, uv_max.y],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

/// 地图区域的网格维度与世界尺寸。
#[derive(Clone, Copy)]
struct MapExtent {
    /// 网格维度。
    grid: UVec2,
    /// 世界尺寸。
    world: Vec2,
}

/// 生成陆地底图。
fn spawn_land_base(commands: &mut Commands, btl: &Btl, resources: &Wc4Resources) {
    let Some(extent) = map_extent(btl) else {
        return;
    };
    let size = extent.world;

    commands.spawn((
        Sprite {
            image: resources.land.clone(),
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 1.0,
            },
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(size.x / 2.0, -size.y / 2.0, Z_LAND_BASE),
    ));
}

/// 生成海洋底图。
fn spawn_sea_base(
    commands: &mut Commands,
    btl: &Btl,
    resources: &Wc4Resources,
    sea_mask: Handle<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<SeaMaterial>,
) {
    let Some(extent) = map_extent(btl) else {
        return;
    };
    let size = extent.world;

    let material = materials.add(SeaMaterial {
        image: resources.sea.clone(),
        sea_mask,
        params: LandSeaParams {
            tile_width: GRID_WIDTH,
            tile_height: GRID_HEIGHT,
            size,
            grid_size: extent.grid,
        },
    });

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::from_size(size))),
        MeshMaterial2d(material),
        Transform::from_xyz(size.x / 2.0, -size.y / 2.0, Z_SEA_BASE),
    ));
}

/// 计算地图区域的网格维度与世界尺寸。
fn map_extent(btl: &Btl) -> Option<MapExtent> {
    let columns = u32::try_from(btl.master.map_width).ok()?;
    let rows = u32::try_from(btl.master.map_height).ok()?;

    if columns == 0 || rows == 0 {
        return None;
    }

    Some(MapExtent {
        grid: UVec2::new(columns, rows),
        world: grid_world_size(columns as usize, rows as usize),
    })
}

/// 生成海洋遮罩纹理。
///
/// 每个像素对应一个地块：水域透明，陆地不透明。
fn build_sea_mask(btl: &Btl, bin: &Bin, images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let master = &btl.master;
    let extent = map_extent(btl)?;
    let pixel_count =
        usize::try_from(extent.grid.x).ok()?.checked_mul(usize::try_from(extent.grid.y).ok()?)?;
    let mut pixels = Vec::with_capacity(pixel_count.checked_mul(4)?);

    for row in 0..master.map_height {
        for column in 0..master.map_width {
            let alpha = master
                .map_capture_x
                .checked_add(column)
                .zip(master.map_capture_y.checked_add(row))
                .and_then(|(x, y)| bin.topography(x, y))
                .map_or(0, |topography| {
                    if topography.r#type == 1 {
                        0
                    } else {
                        255
                    }
                });

            pixels.extend_from_slice(&[0, 0, 0, alpha]);
        }
    }

    let size = Extent3d {
        width: extent.grid.x,
        height: extent.grid.y,
        depth_or_array_layers: 1,
    };

    Some(images.add(Image::new(
        size,
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )))
}

/// 生成城市精灵与城市名称。
fn spawn_cities(
    commands: &mut Commands,
    btl: &Btl,
    bin: &Bin,
    config: &Wc4Config,
    resources: &Wc4Resources,
    image_xml: &Assets<ImageXml>,
) {
    let Some(buildings) = image_xml.get(&resources.buildings.image_xml) else {
        return;
    };

    let frames = city_frames(buildings);
    let width = match usize::try_from(bin.info.width) {
        Ok(width) if width > 0 => width,
        _ => return,
    };
    let text_font =
        TextFont::from_font_size(CITY_LABEL_FONT_SIZE).with_font(resources.font.clone());

    for city in &btl.city_vec {
        let Ok(index) = usize::try_from(city.coordinate) else {
            continue;
        };
        let grid = index_to_grid(index, width);
        let Some(local_x) = grid.x.checked_sub(btl.master.map_capture_x) else {
            continue;
        };
        let Some(local_y) = grid.y.checked_sub(btl.master.map_capture_y) else {
            continue;
        };
        let position = grid_to_world(local_x, local_y);

        if let Some(frame) = frames.get(&city_asset_key(city)) {
            let offset = frame.center_offset();
            commands.spawn((
                Sprite {
                    image: resources.buildings.image.clone(),
                    image_mode: SpriteImageMode::Tiled {
                        tile_x: true,
                        tile_y: true,
                        stretch_value: 1.0,
                    },
                    custom_size: Some(Vec2::new(frame.width, frame.height)),
                    rect: Some(frame.rect()),
                    ..default()
                },
                Transform::from_xyz(position.x + offset.x, position.y + offset.y, Z_CITY),
            ));
        }

        if let Some(name) = config.city_names.name(city.name) {
            commands.spawn((
                CityName,
                Text2d::new(name),
                text_font.clone(),
                TextLayout::justify(Justify::Center),
                Transform::from_xyz(position.x, position.y - CITY_LABEL_OFFSET, Z_CITY),
            ));
        }
    }
}

/// 构建城市外观的图片帧查找表。
fn city_frames(image_xml: &ImageXml) -> HashMap<(&str, u8, u8), ImageFrame> {
    image_xml
        .image_infos
        .iter()
        .filter_map(|info| {
            let key = city_frame_key(&info.name)?;

            Some((key, ImageFrame::from_info(info)))
        })
        .collect()
}

/// 解析城市图片名称，返回前缀、等级与外观编号。
///
/// 名称格式为 `prefix_level_appearance.png`，省略外观编号时视为 `1`。
fn city_frame_key(name: &str) -> Option<(&str, u8, u8)> {
    let base = name.strip_suffix(".png")?;
    let mut parts = base.split('_');

    let prefix = parts.next().filter(|part| !part.is_empty())?;
    let level = parse_number(parts.next())?;
    let appearance = match parts.next() {
        Some(part) => parse_number(Some(part))?,
        None => 1,
    };

    if parts.next().is_some() {
        return None;
    }

    Some((prefix, level, appearance))
}

/// 解析 `u8` 范围内的十进制数字。
fn parse_number(part: Option<&str>) -> Option<u8> {
    let part = part?;

    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    part.parse().ok()
}

/// 返回城市使用的图集键。
fn city_asset_key(city: &City) -> (&'static str, u8, u8) {
    let (kind, id) = if city.wonder == 0 {
        ("building", city.level)
    } else {
        ("capital", city.wonder)
    };

    // 超出图集范围的等级映射到最后一个可用变体。
    let id = match id {
        16..=20 => 15,
        32..=34 => 31,
        _ => id,
    };

    (kind, id, city.appearance)
}

/// 随相机缩放保持城市名称的可读尺寸。
pub(crate) fn update_city_label_scale(
    camera: Single<&Projection, With<MapCamera>>,
    mut labels: Query<&mut Transform, With<CityName>>,
) {
    let Projection::Orthographic(orthographic) = camera.into_inner() else {
        return;
    };
    let scale = orthographic.scale.max(1.0);

    for mut transform in &mut labels {
        transform.scale = Vec3::splat(scale);
    }
}

/// 将地块指示图标移动到鼠标所在的地块。
pub(crate) fn update_tile_cursor(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<MapCamera>>,
    mut cursor: Single<&mut Transform, With<TileCursor>>,
) {
    let (camera, camera_transform) = camera.into_inner();
    let Some(viewport_position) = window.cursor_position() else {
        return;
    };
    let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, viewport_position)
    else {
        return;
    };

    let grid = world_to_grid(world_position);
    let center = grid_to_world(grid.x, grid.y);
    let z = cursor.translation.z;

    cursor.translation = Vec3::new(center.x, center.y, z);
}

/// 地形图集材质。
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct InstanceMaterial {
    /// 图集纹理。
    #[texture(0)]
    #[sampler(1)]
    image: Handle<Image>,
}

impl Material2d for InstanceMaterial {
    fn fragment_shader() -> ShaderRef {
        INSTANCE_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// 海洋材质的着色器参数。
#[derive(Clone, Copy, Debug, ShaderType)]
pub(crate) struct LandSeaParams {
    /// 单个网格的像素宽度。
    tile_width: f32,
    /// 单个网格的像素高度。
    tile_height: f32,
    /// 地图区域的世界尺寸。
    size: Vec2,
    /// 地图区域的网格维度。
    grid_size: UVec2,
}

/// 海洋底图材质。
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct SeaMaterial {
    /// 海洋纹理。
    #[texture(0)]
    #[sampler(1)]
    image: Handle<Image>,
    /// 海洋遮罩纹理。
    #[texture(2)]
    sea_mask: Handle<Image>,
    /// 着色器参数。
    #[uniform(3)]
    params: LandSeaParams,
}

impl Material2d for SeaMaterial {
    fn fragment_shader() -> ShaderRef {
        SEA_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        // 片元按遮罩 alpha 丢弃，底图需要透明混合。
        AlphaMode2d::Blend
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn city_frame_key_parses_level_and_default_appearance() {
        assert_eq!(city_frame_key("building_15.png"), Some(("building", 15, 1)));
    }

    #[test]
    fn city_frame_key_parses_explicit_appearance() {
        assert_eq!(city_frame_key("capital_3_2.png"), Some(("capital", 3, 2)));
    }

    #[test]
    fn city_frame_key_rejects_irregular_names() {
        assert_eq!(city_frame_key("building_15b.png"), None);
        assert_eq!(city_frame_key("building_15.gif"), None);
        assert_eq!(city_frame_key("building_.png"), None);
        assert_eq!(city_frame_key("building_15_1_extra.png"), None);
        assert_eq!(city_frame_key("_15.png"), None);
        assert_eq!(city_frame_key("building_256.png"), None);
    }

    #[test]
    fn city_asset_key_maps_out_of_range_levels() {
        let city = City {
            level: 18,
            wonder: 0,
            appearance: 2,
            ..default()
        };
        assert_eq!(city_asset_key(&city), ("building", 15, 2));

        let city = City {
            level: 4,
            wonder: 33,
            appearance: 0,
            ..default()
        };
        assert_eq!(city_asset_key(&city), ("capital", 31, 0));
    }
}
