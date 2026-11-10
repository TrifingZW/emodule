//! 地图相机与指针控制。

use bevy::{
    input::mouse::MouseScrollUnit,
    picking::{
        events::{PointerDrag, PointerScroll},
        pointer::PointerButton,
    },
    prelude::*,
    ui_widgets::Activate,
};

/// 标记负责渲染地图的相机。
#[derive(Component, Clone, Default)]
#[require(CameraZoom)]
pub(crate) struct MapCamera;

/// 相机缩放缓动的目标值。
#[derive(Component, Clone)]
pub(crate) struct CameraZoom {
    target: f32,
}

impl Default for CameraZoom {
    fn default() -> Self {
        Self {
            target: DEFAULT_ZOOM,
        }
    }
}

/// 最低正交缩放值，对应最近的缩放级别。
const MIN_ZOOM: f32 = 0.1;

/// 最高正交缩放值，对应最远的缩放级别。
const MAX_ZOOM: f32 = 10.0;

/// 默认正交缩放值。
const DEFAULT_ZOOM: f32 = 1.0;

/// 鼠标滚轮每行的缩放量。
const LINE_ZOOM: f32 = 0.1;

/// 鼠标滚轮每像素的缩放量。
const PIXEL_ZOOM: f32 = 0.002;

/// 缩放缓动速率，值越大跟随目标越快。
const ZOOM_SMOOTHING: f32 = 15.0;

/// 缩放与目标的差值低于该阈值时直接对齐，避免无意义的持续更新。
const ZOOM_SNAP_EPSILON: f32 = 1e-4;

/// 按住鼠标中键拖拽时平移地图相机。
pub(crate) fn pan_camera(
    drag: On<PointerDrag>,
    camera: Single<(&mut Transform, &mut Projection), With<MapCamera>>,
) {
    if drag.button != PointerButton::Secondary || !drag.delta.is_finite() {
        return;
    }

    let (mut transform, projection) = camera.into_inner();
    let Projection::Orthographic(orthographic) = &*projection else {
        return;
    };

    // 屏幕坐标的 y 轴向下，世界坐标的 y 轴向上。
    transform.translation.x -= drag.delta.x * orthographic.scale;
    transform.translation.y += drag.delta.y * orthographic.scale;
}

/// 响应鼠标滚轮滚动缩放地图相机。
pub(crate) fn zoom_camera(
    scroll: On<PointerScroll>,
    mut zoom: Single<&mut CameraZoom, With<MapCamera>>,
) {
    let Some(amount) = zoom_amount(scroll.unit, scroll.y) else {
        return;
    };

    // 基于目标值累积缩放，保证缓动期间的连续滚动不会丢失增量。
    zoom.target = (zoom.target * (-amount).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
}

/// 每帧将正交缩放指数缓动到目标值。
pub(crate) fn smooth_zoom(
    time: Res<Time>,
    camera: Single<(&mut Projection, &mut CameraZoom), With<MapCamera>>,
) {
    let (mut projection, zoom) = camera.into_inner();
    let Projection::Orthographic(orthographic) = &mut *projection else {
        return;
    };

    let delta = zoom.target - orthographic.scale;
    if delta.abs() < ZOOM_SNAP_EPSILON {
        orthographic.scale = zoom.target;
        return;
    }

    // 指数缓动与帧率无关，不同刷新率下收敛速度一致。
    let factor = 1.0 - (-ZOOM_SMOOTHING * time.delta_secs()).exp();
    orthographic.scale += delta * factor;
}

/// 恢复地图相机的位置与缩放。
pub(crate) fn reset_camera(
    _: On<Activate>,
    camera: Single<(&mut Transform, &mut Projection, &mut CameraZoom), With<MapCamera>>,
) {
    let (mut transform, mut projection, mut zoom) = camera.into_inner();
    transform.translation = Vec3::ZERO;
    zoom.target = DEFAULT_ZOOM;

    if let Projection::Orthographic(orthographic) = &mut *projection {
        orthographic.scale = DEFAULT_ZOOM;
    }
}

/// 将滚轮增量转换为缩放量。
///
/// 当增量为零或非有限值时返回 `None`。
fn zoom_amount(unit: MouseScrollUnit, delta: f32) -> Option<f32> {
    if !delta.is_finite() {
        return None;
    }

    let amount = match unit {
        MouseScrollUnit::Line => delta * LINE_ZOOM,
        MouseScrollUnit::Pixel => delta * PIXEL_ZOOM,
    };

    (amount != 0.0).then_some(amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_amount_scales_line_deltas() {
        let amount = zoom_amount(MouseScrollUnit::Line, 10.0).unwrap();
        assert!((amount - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn zoom_amount_scales_pixel_deltas() {
        let amount = zoom_amount(MouseScrollUnit::Pixel, 500.0).unwrap();
        assert!((amount - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn zoom_amount_ignores_zero_and_non_finite_deltas() {
        assert_eq!(zoom_amount(MouseScrollUnit::Line, 0.0), None);
        assert_eq!(zoom_amount(MouseScrollUnit::Pixel, 0.0), None);
        assert_eq!(zoom_amount(MouseScrollUnit::Line, f32::NAN), None);
        assert_eq!(zoom_amount(MouseScrollUnit::Pixel, f32::INFINITY), None);
    }
}
