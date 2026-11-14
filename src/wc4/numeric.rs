//! 数值字段写入时的类型范围处理。

/// 将编辑值截断到 `i8` 范围。
pub(super) fn clamp_i8(value: i32) -> i8 {
    value.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8
}

/// 将编辑值截断到 `u8` 范围。
pub(super) fn clamp_u8(value: i32) -> u8 {
    value.clamp(i32::from(u8::MIN), i32::from(u8::MAX)) as u8
}

/// 将编辑值截断到 `u16` 范围。
pub(super) fn clamp_u16(value: i32) -> u16 {
    value.clamp(i32::from(u16::MIN), i32::from(u16::MAX)) as u16
}

/// 将编辑值截断到 `i16` 范围。
pub(super) fn clamp_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// 保留 `i32` 原值。
pub(super) fn clamp_i32(value: i32) -> i32 {
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_saturate_at_the_type_bounds() {
        assert_eq!(clamp_i8(i32::MAX), i8::MAX);
        assert_eq!(clamp_i8(i32::MIN), i8::MIN);
        assert_eq!(clamp_u8(-1), 0);
        assert_eq!(clamp_u8(300), u8::MAX);
        assert_eq!(clamp_u16(-1), 0);
        assert_eq!(clamp_u16(90_000), u16::MAX);
        assert_eq!(clamp_i16(40_000), i16::MAX);
        assert_eq!(clamp_i16(-40_000), i16::MIN);
        assert_eq!(clamp_i32(i32::MIN), i32::MIN);
    }
}
