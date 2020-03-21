use std::intrinsics::unlikely;

macro_rules! impl_write_unsigned_leb128 {
    ($fn_name:ident, $int_ty:ident) => {
        #[inline]
        pub fn $fn_name(out: &mut Vec<u8>, mut value: $int_ty) {
            if value < u32::max_value() as $int_ty {
                write_u32_leb128(out, value as u32);
            } else {
                loop {
                    if value < 0x80 {
                        out.push(value as u8);
                        break;
                    } else {
                        out.push(((value & 0x7f) | 0x80) as u8);
                        value >>= 7;
                    }
                }
            }
        }
    };
}

impl_write_unsigned_leb128!(write_u64_leb128, u64);
impl_write_unsigned_leb128!(write_u128_leb128, u128);
impl_write_unsigned_leb128!(write_usize_leb128, usize);

#[inline]
pub fn write_u16_leb128(out: &mut Vec<u8>, value: u16) {
    write_u32_leb128(out, value.into());
}

#[inline]
pub fn write_u32_leb128(out: &mut Vec<u8>, value: u32) {
    let hi = 0x8080_8080_8080_8080;
    let split = unsafe { std::arch::x86_64::_pdep_u64(value as u64, !hi) };
    let leading0s = (split | 1).leading_zeros();
    let tags = (!0 >> leading0s) & hi;
    let leb128 = split | tags;
    let bytes = 8 - (leading0s / 8);
    out.extend_from_slice(&leb128.to_le_bytes()[..bytes as usize]);
}

macro_rules! impl_read_unsigned_leb128 {
    ($fn_name:ident, $int_ty:ident) => {
        #[inline]
        pub fn $fn_name(slice: &[u8]) -> ($int_ty, usize) {
            let mut result = 0;
            let mut shift = 0;
            let mut position = 0;
            loop {
                let byte = slice[position];
                position += 1;
                if (byte & 0x80) == 0 {
                    result |= (byte as $int_ty) << shift;
                    return (result, position);
                } else {
                    result |= ((byte & 0x7F) as $int_ty) << shift;
                }
                shift += 7;
            }
        }
    };
}

fn read_slow(slice: &[u8]) -> (u128, usize) {
    let mut result = 0;
    let mut shift = 0;
    let mut position = 0;
    loop {
        let byte = slice[position];
        position += 1;
        if (byte & 0x80) == 0 {
            result |= (byte as u128) << shift;
            return (result, position);
        } else {
            result |= ((byte & 0x7F) as u128) << shift;
        }
        shift += 7;
    }
}

#[inline]
pub fn read_u64_leb128(slice: &[u8]) -> (u64, usize) {
    if unlikely(slice.len() < 8) {
        let (v, count) = read_slow(slice);
        (v as u64, count)
    } else {
        // Up to 8 encoded bytes can be decoded at once via SIMD.
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&slice[..8]);

        todo!()
    }
}

impl_read_unsigned_leb128!(read_u16_leb128, u16);
impl_read_unsigned_leb128!(read_u32_leb128, u32);
impl_read_unsigned_leb128!(read_u128_leb128, u128);
impl_read_unsigned_leb128!(read_usize_leb128, usize);

#[inline]
/// encodes an integer using signed leb128 encoding and stores
/// the result using a callback function.
///
/// The callback `write` is called once for each position
/// that is to be written to with the byte to be encoded
/// at that position.
pub fn write_signed_leb128_to<W>(mut value: i128, mut write: W)
where
    W: FnMut(u8),
{
    loop {
        let mut byte = (value as u8) & 0x7f;
        value >>= 7;
        let more =
            !(((value == 0) && ((byte & 0x40) == 0)) || ((value == -1) && ((byte & 0x40) != 0)));

        if more {
            byte |= 0x80; // Mark this byte to show that more bytes will follow.
        }

        write(byte);

        if !more {
            break;
        }
    }
}

#[inline]
pub fn write_signed_leb128(out: &mut Vec<u8>, value: i128) {
    write_signed_leb128_to(value, |v| out.push(v))
}

#[inline]
pub fn read_signed_leb128(data: &[u8], start_position: usize) -> (i128, usize) {
    let mut result = 0;
    let mut shift = 0;
    let mut position = start_position;
    let mut byte;

    loop {
        byte = data[position];
        position += 1;
        result |= i128::from(byte & 0x7F) << shift;
        shift += 7;

        if (byte & 0x80) == 0 {
            break;
        }
    }

    if (shift < 64) && ((byte & 0x40) != 0) {
        // sign extend
        result |= -(1 << shift);
    }

    (result, position - start_position)
}
