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
pub(crate) struct MapCamera;

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

/// 按住鼠标中键拖拽时平移地图相机。
pub(crate) fn pan_camera(
    drag: On<PointerDrag>,
    camera: Single<(&mut Transform, &mut Projection), With<MapCamera>>,
) {
    if drag.button != PointerButton::Middle || !drag.delta.is_finite() {
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
    mut camera: Single<&mut Projection, With<MapCamera>>,
) {
    let Some(amount) = zoom_amount(scroll.unit, scroll.y) else {
        return;
    };

    let Projection::Orthographic(orthographic) = &mut **camera else {
        return;
    };

    orthographic.scale = (orthographic.scale * (-amount).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
}

/// 恢复地图相机的位置与缩放。
pub(crate) fn reset_camera(
    _: On<Activate>,
    camera: Single<(&mut Transform, &mut Projection), With<MapCamera>>,
) {
    let (mut transform, mut projection) = camera.into_inner();
    transform.translation = Vec3::ZERO;

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
