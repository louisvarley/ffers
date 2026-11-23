//! The original's packed short floats, used for the orbital values of a
//! system's objects: a dword whose low word is a signed 15-bit mantissa and
//! whose high word is a signed exponent. F1482 multiplies two, F1494 turns one
//! into an integer and F1515 multiplies a 16.16 value by an integer.

fn mantissa(value: u32) -> i32 {
    i32::from(value as u16 as i16)
}

fn exponent(value: u32) -> i32 {
    i32::from((value >> 16) as u16 as i16)
}

/// F1482: the product of two short floats: the mantissas multiplied and shifted
/// right fifteen places, the exponents added (16 bits, wrapping).
pub fn multiply(a: u32, b: u32) -> u32 {
    let product = (mantissa(a) * mantissa(b)) >> 15;
    let exponent = ((a >> 16) as u16).wrapping_add((b >> 16) as u16);
    u32::from(exponent) << 16 | u32::from(product as u16)
}

/// F1494: the short float as an integer: the sign-extended mantissa shifted
/// (logically, by `cl` modulo 32) right by `15 - exponent`, or left by
/// `exponent - 15` from exponent 16 to 32, and `-1` beyond that.
pub fn to_integer(value: u32) -> i32 {
    let (mantissa, exponent) = (mantissa(value), exponent(value));
    if exponent <= 0xf {
        ((mantissa as u32) >> ((15 - exponent) as u32 & 31)) as i32
    } else if exponent <= 0x20 {
        ((mantissa as u32) << ((exponent - 15) as u32 & 31)) as i32
    } else {
        -1
    }
}

/// F1515: a 16.16 fixed value times an integer: the low half's product shifted
/// down sixteen places plus the high half's product.
pub fn scale(fixed: u32, factor: i32) -> u32 {
    let low = (fixed & 0xffff).wrapping_mul(factor as u32) >> 16;
    let high = (fixed >> 16).wrapping_mul(factor as u32);
    low.wrapping_add(high)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn products_and_conversions_follow_the_original() {
        // 0x4000 is one half; two halves multiply to a quarter (0x2000).
        assert_eq!(multiply(0x4000, 0x4000), 0x2000);
        // Exponents add: (0x4000, 3) * (0x4000, -1) = (0x2000, 2).
        assert_eq!(multiply(3 << 16 | 0x4000, 0xffff_4000), 2 << 16 | 0x2000);
        // Exponent 15 leaves the mantissa; below it shifts right.
        assert_eq!(to_integer(15 << 16 | 1000), 1000);
        assert_eq!(to_integer(13 << 16 | 1000), 250);
        assert_eq!(to_integer(17 << 16 | 3), 12);
        assert_eq!(to_integer(0x30 << 16 | 3), -1);
        assert_eq!(scale(0x0001_8000, 10), 15);
    }
}
