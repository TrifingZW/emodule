//! WC4 六边形地图渲染。

use std::collections::{HashMap, HashSet};

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
    font::EditorFont,
    state::SelectedTile,
    wc4::{
        bin::Bin,
        btl::Btl,
        btl_data::City,
        config::{TerrainXml, Wc4Config},
        grid::{
            GRID_HEIGHT, GRID_WIDTH, cursor_to_grid, grid_to_world, grid_world_size, index_to_grid,
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
/// 选中地块标记的深度。
const Z_TILE_SELECTION: f32 = 10.0;
/// 地块指示图标的深度。
const Z_TILE_CURSOR: f32 = 11.0;
/// 相邻行的深度间隔，保证行优先的稳定排序。
const Z_ROW_BIAS: f32 = 0.001;
/// 相邻列的深度间隔，用于解决同一行内的排序。
const Z_COLUMN_BIAS: f32 = 0.000_001;
/// 地形层（基础 > 装饰 A > 装饰 B）的相对深度。
const Z_TILE_LAYERS: [f32; 3] = [3.0, 2.0, 1.0];
/// 网格覆盖层的深度，位于地块及其颜色层之上、城市与交互标记之下。
pub(crate) const Z_GRID: f32 = 4.5;
/// 省界覆盖层的深度，位于网格之上、城市与交互标记之下。
pub(crate) const Z_PROVINCE_BOUNDARIES: f32 = 4.75;
/// 城市名称与城市精灵的距离。
const CITY_LABEL_OFFSET: f32 = 50.0;
/// 城市名称的字号（逻辑像素）。
const CITY_LABEL_FONT_SIZE: f32 = 16.0;

/// 地形图集着色器路径。
const INSTANCE_SHADER_PATH: &str = "shaders/automatic_instancing.wesl";
/// 海洋遮罩着色器路径。
const SEA_SHADER_PATH: &str = "shaders/land_sea.wesl";

/// 选中标记的着色，用于区分悬停与选中。
const SELECTION_TINT: Color = Color::srgb(1.0, 0.78, 0.25);

/// 标记 WC4 地图已经生成。
#[derive(Resource)]
pub(crate) struct Wc4MapSpawned;

/// 标记城市名称文本。
#[derive(Component)]
pub(crate) struct CityName;

/// 城市实体承担的渲染部分。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CityPart {
    /// 城市外观精灵。
    Sprite,
    /// 城市名称文本。
    Label,
}

/// 标记城市精灵或名称实体，记录其对应的城市下标。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CityObject {
    /// 城市在 `Btl::city_vec` 中的下标。
    pub(crate) index: usize,
    /// 实体承担的渲染部分。
    pub(crate) part: CityPart,
}

/// 标记跟随鼠标的地块指示图标。
#[derive(Component)]
pub(crate) struct TileCursor;

/// 标记当前选中地块的图标。
#[derive(Component)]
pub(crate) struct SelectionMarker;

/// 标记属于某个地块的渲染实体。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileObject {
    /// 所属地块的网格坐标。
    pub(crate) grid: IVec2,
}

/// 待重建渲染的地块集合。
#[derive(Resource, Default)]
pub(crate) struct DirtyTiles(HashSet<IVec2>);

impl DirtyTiles {
    /// 登记需要重建渲染的地块。
    pub(crate) fn mark(&mut self, grid: IVec2) {
        self.0.insert(grid);
    }
}

/// 地块增量刷新所需的渲染资源。
#[derive(Resource)]
pub(crate) struct Wc4Render {
    /// 地形图集。
    terrain: PreparedAtlas,
    /// 植被图集。
    plant: PreparedAtlas,
    /// 地形材质。
    terrain_material: Handle<InstanceMaterial>,
    /// 植被材质。
    plant_material: Handle<InstanceMaterial>,
    /// 海洋遮罩纹理。
    sea_mask: Option<Handle<Image>>,
}

/// 单个地形网格实体所需的组件。
type TileBundle = (TileObject, Mesh2d, MeshMaterial2d<InstanceMaterial>, MeshTag, Transform);

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

/// 地块增量刷新所需的只读资源。
#[derive(SystemParam)]
pub(crate) struct RefreshContext<'w> {
    /// 渲染资源。
    render: Option<Res<'w, Wc4Render>>,
    /// 世界地形数据。
    bin: Option<Res<'w, Bin>>,
    /// 场景数据。
    btl: Option<Res<'w, Btl>>,
    /// 地图配置。
    config: Option<Res<'w, Wc4Config>>,
    /// 地图资源。
    resources: Option<Res<'w, Wc4Resources>>,
    /// 全局字体。
    font: Option<Res<'w, EditorFont>>,
    /// 图集配置资源。
    image_xml: Res<'w, Assets<ImageXml>>,
}

/// 地块增量刷新所需的可写资源。
#[derive(SystemParam)]
pub(crate) struct RefreshAssets<'w> {
    /// 网格资源。
    meshes: ResMut<'w, Assets<Mesh>>,
    /// 纹理资源。
    images: ResMut<'w, Assets<Image>>,
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
    font: Res<EditorFont>,
    mut assets: MapAssets,
) {
    let (Some(bin), Some(btl)) = (bin, btl) else {
        return;
    };

    // 1) 两层地形图集必须同时就绪，否则留到下一帧整体重试。
    let Some(terrain_atlas) = prepare_atlas(
        &config,
        &assets.image_xml,
        &assets.terrain_xml,
        &assets.images,
        &resources.terrain,
    ) else {
        return;
    };
    let Some(plant_atlas) = prepare_atlas(
        &config,
        &assets.image_xml,
        &assets.terrain_xml,
        &assets.images,
        &resources.plant,
    ) else {
        return;
    };

    // 城市图集未就绪时同样等待，避免生成缺少城市的半成品地图。
    if assets.image_xml.get(&resources.buildings.image_xml).is_none() {
        return;
    }

    // 2) 地形、植被与底图。
    let terrain_material = assets.instance_materials.add(InstanceMaterial {
        image: terrain_atlas.image.clone(),
    });
    let plant_material = assets.instance_materials.add(InstanceMaterial {
        image: plant_atlas.image.clone(),
    });

    let terrain_batch =
        tile_batch(&btl, &bin, &terrain_atlas, &terrain_material, &mut assets.meshes);
    let plant_batch = tile_batch(&btl, &bin, &plant_atlas, &plant_material, &mut assets.meshes);
    let tile_count = terrain_batch.len() + plant_batch.len();

    commands.spawn_batch(terrain_batch);
    commands.spawn_batch(plant_batch);
    spawn_land_base(&mut commands, &btl, &resources);

    let sea_mask = build_sea_mask(&btl, &bin, &mut assets.images);
    if let Some(mask) = &sea_mask {
        spawn_sea_base(
            &mut commands,
            &btl,
            &resources,
            mask.clone(),
            &mut assets.meshes,
            &mut assets.sea_materials,
        );
    } else {
        warn!("WC4 海洋遮罩尺寸无效，已跳过海洋层");
    }

    // 3) 城市与交互标记。
    spawn_cities(&mut commands, &btl, &bin, &config, &resources, &font, &assets.image_xml);
    commands.spawn((
        TileCursor,
        Sprite::from_image(resources.cursor.clone()),
        Transform::from_xyz(0.0, 0.0, Z_TILE_CURSOR),
    ));
    commands.spawn((
        SelectionMarker,
        Sprite {
            image: resources.cursor.clone(),
            color: SELECTION_TINT,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, Z_TILE_SELECTION),
        Visibility::Hidden,
    ));

    info!(
        "WC4 地图已生成: {}×{}，{} 个地形网格，{} 个城市",
        btl.master.map_width,
        btl.master.map_height,
        tile_count,
        btl.city_vec.len()
    );

    commands.insert_resource(Wc4Render {
        terrain: terrain_atlas,
        plant: plant_atlas,
        terrain_material,
        plant_material,
        sea_mask,
    });
    commands.insert_resource(Wc4MapSpawned);
}

/// 准备单层地形的图集数据。
///
/// 任一依赖资源尚未加载时返回 `None`。
fn prepare_atlas(
    config: &Wc4Config,
    image_xml: &Assets<ImageXml>,
    terrain_xml: &Assets<TerrainXml>,
    images: &Assets<Image>,
    atlas: &Wc4Atlas,
) -> Option<PreparedAtlas> {
    let image_doc = image_xml.get(&atlas.image_xml)?;
    let terrain_doc = terrain_xml.get(&config.terrain_xml)?;
    let texture = images.get(&atlas.image)?;
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
    btl: &Btl,
    bin: &Bin,
    atlas: &PreparedAtlas,
    material: &Handle<InstanceMaterial>,
    meshes: &mut Assets<Mesh>,
) -> TileBatch {
    let mut batch = Vec::new();

    for row in 0..btl.master.map_height {
        for column in 0..btl.master.map_width {
            tile_layers(btl, bin, atlas, material, IVec2::new(column, row), meshes, &mut batch);
        }
    }

    batch
}

/// 生成指定地块在单个图集中的全部网格实体。
fn tile_layers(
    btl: &Btl,
    bin: &Bin,
    atlas: &PreparedAtlas,
    material: &Handle<InstanceMaterial>,
    grid: IVec2,
    meshes: &mut Assets<Mesh>,
    batch: &mut TileBatch,
) {
    let master = &btl.master;
    let Some(x) = master.map_capture_x.checked_add(grid.x) else {
        return;
    };
    let Some(y) = master.map_capture_y.checked_add(grid.y) else {
        return;
    };
    let Some(index) = bin.index(x, y) else {
        return;
    };
    let Some(topography) = bin.topographies.get(index) else {
        return;
    };

    let position = grid_to_world(grid.x, grid.y);
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
            Vec2::new(f32::from(topography.decoration_a_x), f32::from(topography.decoration_a_y)),
        ),
        (
            i32::from(topography.decoration_b),
            i32::from(topography.decoration_b_id),
            Vec2::new(f32::from(topography.decoration_b_x), f32::from(topography.decoration_b_y)),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let Some(frame) = atlas.frames.get(&(kind, id)) else {
            continue;
        };
        let frame_offset = frame.center_offset() + Vec2::new(offset.x, -offset.y);

        batch.push((
            TileObject {
                grid,
            },
            Mesh2d(meshes.add(quad_mesh(*frame, atlas.texture_size))),
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

/// 计算海洋遮罩像素数据。
///
/// 每个像素对应一个地块：水域透明，陆地不透明。
fn sea_mask_pixels(btl: &Btl, bin: &Bin) -> Option<Vec<u8>> {
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

    Some(pixels)
}

/// 生成海洋遮罩纹理。
fn build_sea_mask(btl: &Btl, bin: &Bin, images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let pixels = sea_mask_pixels(btl, bin)?;
    let extent = map_extent(btl)?;
    let size = Extent3d {
        width: extent.grid.x,
        height: extent.grid.y,
        depth_or_array_layers: 1,
    };

    // 遮罩需要保留 CPU 数据，以便地形编辑后原地刷新纹理。
    Some(images.add(Image::new(
        size,
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )))
}

/// 用地形数据刷新已有的海洋遮罩纹理。
fn update_sea_mask(btl: &Btl, bin: &Bin, mask: &Handle<Image>, images: &mut Assets<Image>) {
    let Some(pixels) = sea_mask_pixels(btl, bin) else {
        return;
    };
    let Some(mut image) = images.get_mut(mask) else {
        return;
    };

    image.data = Some(pixels);
}

/// 城市渲染共用的图集帧、字体与地图数据。
struct CityScene<'a> {
    btl: &'a Btl,
    config: &'a Wc4Config,
    resources: &'a Wc4Resources,
    frames: HashMap<(&'a str, u8, u8), ImageFrame>,
    font: TextFont,
    width: usize,
}

impl<'a> CityScene<'a> {
    /// 构建城市渲染上下文。
    ///
    /// 图片图集尚未加载或地图宽度无效时返回 `None`。
    fn new(
        btl: &'a Btl,
        bin: &Bin,
        config: &'a Wc4Config,
        resources: &'a Wc4Resources,
        font: &EditorFont,
        image_xml: &'a Assets<ImageXml>,
    ) -> Option<Self> {
        let buildings = image_xml.get(&resources.buildings.image_xml)?;
        let width = usize::try_from(bin.info.width).ok().filter(|width| *width > 0)?;

        Some(Self {
            btl,
            config,
            resources,
            frames: city_frames(buildings),
            font: font.text_font(CITY_LABEL_FONT_SIZE),
            width,
        })
    }

    /// 返回城市所在的地块坐标。
    fn grid(&self, city: &City) -> Option<IVec2> {
        let index = usize::try_from(city.coordinate).ok()?;
        let map = index_to_grid(index, self.width);

        local_grid(self.btl, map.x, map.y)
    }

    /// 返回城市外观对应的图片帧。
    fn frame(&self, city: &City) -> Option<&ImageFrame> {
        self.frames.get(&city_asset_key(city))
    }

    /// 返回城市名称文本。
    fn name(&self, city: &City) -> Option<&str> {
        self.config.city_names.name(city.name)
    }

    /// 构建城市外观精灵。
    fn sprite(&self, frame: &ImageFrame) -> Sprite {
        Sprite {
            image: self.resources.buildings.image.clone(),
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 1.0,
            },
            custom_size: Some(Vec2::new(frame.width, frame.height)),
            rect: Some(frame.rect()),
            ..default()
        }
    }
}

/// 生成城市精灵与城市名称。
fn spawn_cities(
    commands: &mut Commands,
    btl: &Btl,
    bin: &Bin,
    config: &Wc4Config,
    resources: &Wc4Resources,
    font: &EditorFont,
    image_xml: &Assets<ImageXml>,
) {
    let Some(scene) = CityScene::new(btl, bin, config, resources, font, image_xml) else {
        return;
    };

    for (index, city) in btl.city_vec.iter().enumerate() {
        let Some(grid) = scene.grid(city) else {
            continue;
        };

        spawn_city(commands, index, city, grid, &scene);
    }
}

/// 生成单个城市在指定地块上的精灵与名称。
fn spawn_city(commands: &mut Commands, index: usize, city: &City, grid: IVec2, scene: &CityScene) {
    let position = grid_to_world(grid.x, grid.y);

    if let Some(bundle) = city_sprite_bundle(index, city, grid, scene) {
        commands.spawn(bundle);
    }

    if let Some(name) = scene.name(city) {
        commands.spawn((
            CityObject {
                index,
                part: CityPart::Label,
            },
            CityName,
            Text2d::new(name),
            scene.font.clone(),
            TextLayout::justify(Justify::Center),
            Transform::from_xyz(position.x, position.y - CITY_LABEL_OFFSET, Z_CITY),
        ));
    }
}

/// 构建城市精灵实体所需的组件。
///
/// 图集缺少对应外观时返回 `None`。
fn city_sprite_bundle(
    index: usize,
    city: &City,
    grid: IVec2,
    scene: &CityScene,
) -> Option<(CityObject, Sprite, Transform)> {
    let frame = scene.frame(city)?;
    let position = grid_to_world(grid.x, grid.y);
    let offset = frame.center_offset();

    Some((
        CityObject {
            index,
            part: CityPart::Sprite,
        },
        scene.sprite(frame),
        Transform::from_xyz(position.x + offset.x, position.y + offset.y, Z_CITY),
    ))
}

/// 将地图绝对坐标换算为地图内的网格坐标。
fn local_grid(btl: &Btl, x: i32, y: i32) -> Option<IVec2> {
    Some(IVec2::new(
        x.checked_sub(btl.master.map_capture_x)?,
        y.checked_sub(btl.master.map_capture_y)?,
    ))
}

/// 重建被编辑过的地块的渲染实体。
pub(crate) fn refresh_tiles(
    mut dirty: ResMut<DirtyTiles>,
    context: RefreshContext,
    mut assets: RefreshAssets,
    objects: Query<(Entity, &TileObject, Option<&Mesh2d>)>,
    cities: Query<(Entity, &CityObject)>,
    mut commands: Commands,
) {
    if dirty.0.is_empty() {
        return;
    }

    // 地图尚未生成时保留待刷新列表，等待渲染资源就绪。
    let (Some(render), Some(bin), Some(btl)) =
        (context.render.as_deref(), context.bin.as_deref(), context.btl.as_deref())
    else {
        return;
    };

    let grids = std::mem::take(&mut dirty.0);

    // 城市精灵不带 `TileObject`，由 `refresh_cities` 就地更新；
    // 若在此销毁再重建，精灵网格系统针对 `Added<Sprite>` 的延迟插入会命中已释放的实体。
    for (entity, object, mesh) in &objects {
        if !grids.contains(&object.grid) {
            continue;
        }

        // 每个地块层拥有独立的网格资源，销毁实体时一并释放。
        if let Some(mesh) = mesh {
            assets.meshes.remove(mesh.id());
        }
        commands.entity(entity).try_despawn();
    }

    let mut batch = TileBatch::new();
    for grid in &grids {
        tile_layers(
            btl,
            bin,
            &render.terrain,
            &render.terrain_material,
            *grid,
            &mut assets.meshes,
            &mut batch,
        );
        tile_layers(
            btl,
            bin,
            &render.plant,
            &render.plant_material,
            *grid,
            &mut assets.meshes,
            &mut batch,
        );
    }
    commands.spawn_batch(batch);

    if let (Some(config), Some(resources), Some(font)) =
        (context.config.as_deref(), context.resources.as_deref(), context.font.as_deref())
        && let Some(scene) = CityScene::new(btl, bin, config, resources, font, &context.image_xml)
    {
        let existing =
            cities.iter().map(|(entity, city)| ((city.index, city.part), entity)).collect();
        refresh_cities(&mut commands, &scene, &grids, &existing);
    }

    if let Some(mask) = &render.sea_mask {
        update_sea_mask(btl, bin, mask, &mut assets.images);
    }
}

/// 更新指定地块上的城市实体。
///
/// 已存在的实体只更新组件，避免重复 `Added<Sprite>` 与精灵网格系统的延迟插入竞争。
fn refresh_cities(
    commands: &mut Commands,
    scene: &CityScene,
    grids: &HashSet<IVec2>,
    existing: &HashMap<(usize, CityPart), Entity>,
) {
    for (index, city) in scene.btl.city_vec.iter().enumerate() {
        let Some(grid) = scene.grid(city) else {
            continue;
        };
        if !grids.contains(&grid) {
            continue;
        }

        refresh_city(commands, index, city, grid, scene, existing);
    }
}

/// 更新单个城市在指定地块上的精灵与名称。
///
/// 城市实体一旦生成就不再销毁，缺失外观或名称时仅隐藏，
/// 避开精灵网格系统对 `Added<Sprite>` 的延迟插入与销毁竞争。
fn refresh_city(
    commands: &mut Commands,
    index: usize,
    city: &City,
    grid: IVec2,
    scene: &CityScene,
    existing: &HashMap<(usize, CityPart), Entity>,
) {
    let sprite = existing.get(&(index, CityPart::Sprite)).copied();
    match (city_sprite_bundle(index, city, grid, scene), sprite) {
        (Some(bundle), Some(entity)) => {
            let (_, sprite, transform) = bundle;
            commands.entity(entity).try_insert((sprite, transform, Visibility::Visible));
        }
        (Some(bundle), None) => {
            commands.spawn(bundle);
        }
        (None, Some(entity)) => {
            commands.entity(entity).try_insert(Visibility::Hidden);
        }
        (None, None) => {}
    }

    let label = existing.get(&(index, CityPart::Label)).copied();
    match (scene.name(city), label) {
        (Some(name), Some(entity)) => {
            commands.entity(entity).try_insert((Text2d::new(name), Visibility::Visible));
        }
        (Some(name), None) => {
            let position = grid_to_world(grid.x, grid.y);
            commands.spawn((
                CityObject {
                    index,
                    part: CityPart::Label,
                },
                CityName,
                Text2d::new(name),
                scene.font.clone(),
                TextLayout::justify(Justify::Center),
                Transform::from_xyz(position.x, position.y - CITY_LABEL_OFFSET, Z_CITY),
            ));
        }
        (None, Some(entity)) => {
            commands.entity(entity).try_insert(Visibility::Hidden);
        }
        (None, None) => {}
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
    // 光标换算是悬停与点选共用的唯一入口。
    let Some(grid) = cursor_to_grid(&window, camera, camera_transform) else {
        return;
    };
    let center = grid_to_world(grid.x, grid.y);
    let z = cursor.translation.z;

    cursor.translation = Vec3::new(center.x, center.y, z);
}

/// 将选中标记移动到当前选中的地块。
pub(crate) fn update_selection_marker(
    selected: Res<SelectedTile>,
    mut marker: Query<(&mut Transform, &mut Visibility), With<SelectionMarker>>,
) {
    let Ok((mut transform, mut visibility)) = marker.single_mut() else {
        return;
    };

    let Some(grid) = selected.get() else {
        *visibility = Visibility::Hidden;
        return;
    };

    let center = grid_to_world(grid.x, grid.y);
    transform.translation.x = center.x;
    transform.translation.y = center.y;
    *visibility = Visibility::Visible;
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
