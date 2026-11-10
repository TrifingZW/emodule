//! WC4 二进制数据的通用解码工具。

use bincode::{Decode, config};

use crate::wc4::Wc4Error;

/// 使用 WC4 固定的整数编码配置解码单个值。
///
/// 解码成功后会将 `buffer` 前移到剩余数据处。
pub(crate) fn decode<T: Decode<()>>(buffer: &mut &[u8]) -> Result<T, Wc4Error> {
    let (value, bytes_read) =
        bincode::decode_from_slice(buffer, config::standard().with_fixed_int_encoding())?;
    *buffer = buffer.get(bytes_read..).ok_or(Wc4Error::Invalid("解码位置超出数据长度"))?;

    Ok(value)
}

/// 解码固定数量的连续记录。
///
/// 每条记录至少占用一个字节，因此记录数超过剩余字节数时直接判定为非法，
/// 避免损坏的数据触发超额内存分配。
pub(crate) fn decode_sequence<T: Decode<()>>(
    buffer: &mut &[u8],
    count: usize,
) -> Result<Vec<T>, Wc4Error> {
    if count > buffer.len() {
        return Err(Wc4Error::Invalid("记录数量超出剩余数据长度"));
    }

    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decode(buffer)?);
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_sequence_reads_the_requested_number_of_values() {
        let bytes =
            bincode::encode_to_vec([1_u8, 2, 3], config::standard().with_fixed_int_encoding())
                .unwrap();
        let mut buffer = bytes.as_slice();

        let values: Vec<u8> = decode_sequence(&mut buffer, 3).unwrap();

        assert_eq!(values, [1, 2, 3]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn decode_sequence_rejects_counts_beyond_the_buffer() {
        let bytes = bincode::encode_to_vec([1_u8, 2], config::standard().with_fixed_int_encoding())
            .unwrap();
        let mut buffer = bytes.as_slice();

        assert!(matches!(decode_sequence::<u8>(&mut buffer, 3), Err(Wc4Error::Invalid(_))));
    }

    #[test]
    fn decode_reports_truncated_data() {
        let mut buffer = [1_u8, 2].as_slice();

        assert!(matches!(decode::<u32>(&mut buffer), Err(Wc4Error::Decode(_))));
    }
}
