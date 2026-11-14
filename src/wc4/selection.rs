//! 地图地块的点选交互。

use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};

use crate::{
    camera::MapCamera,
    state::SelectedTile,
    wc4::{bin::Bin, btl::Btl, grid::cursor_to_grid, tile::tile_summary},
};

/// 左键点击视口时选中光标所在地块。
///
/// 点击地图范围外或不存在地形数据的位置时清除选中。坐标换算与悬停指示
/// 共用 `cursor_to_grid`，有效性判定与检查器共用 `tile_summary`。
pub(crate) fn select_tile_on_click(
    click: On<PointerClick>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<MapCamera>>,
    btl: Option<Res<Btl>>,
    bin: Option<Res<Bin>>,
    mut selected: ResMut<SelectedTile>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    // 地图数据尚未就绪时保持现有选中不变。
    let (Some(btl), Some(bin)) = (btl, bin) else {
        return;
    };

    let (camera, camera_transform) = camera.into_inner();
    let next = cursor_to_grid(&window, camera, camera_transform)
        .filter(|grid| tile_summary(&btl, &bin, *grid).is_some())
        .map_or_else(SelectedTile::default, SelectedTile::at);

    selected.set_if_neq(next);
}
