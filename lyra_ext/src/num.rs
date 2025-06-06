#[inline]
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub const fn usize_as_i64(n: usize) -> i64 {
    n as i64
}

#[inline]
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_as_u8(n: usize) -> u8 {
    n as u8
}

#[inline]
#[must_use]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub const fn i64_as_usize(n: i64) -> usize {
    n as usize
}

#[inline]
#[must_use]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub const fn i64_as_u16(n: i64) -> u16 {
    n as u16
}

#[inline]
#[must_use]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub const fn f64_as_u32(n: f64) -> u32 {
    n as u32
}

#[inline]
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub const fn usize_as_f64(n: usize) -> f64 {
    n as f64
}

#[inline]
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub const fn f64_as_isize(n: f64) -> isize {
    n as isize
}

#[cfg(test)]
mod test {
    use hexf::hexf64;
    use rstest::rstest;

    use crate::num::{
        f64_as_isize, f64_as_u32, i64_as_u16, i64_as_usize, usize_as_f64, usize_as_i64, usize_as_u8,
    };

    #[rstest]
    #[case(0x0, 0x0)]
    #[case(0x1, 0x1)]
    #[case(0x3, 0x3)]
    #[case(0xF, 0xF)]
    #[case(0xFF, 0xFF)]
    #[case(0xFFFF, 0xFFFF)]
    #[case(0xFFFF_FFFF, 0xFFFF_FFFF)]
    fn usize_as_i64_trivial(#[case] input: usize, #[case] expected: i64) {
        assert_eq!(usize_as_i64(input), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(0x1_0000_0000, 0x1_0000_0000)]
    #[case(0x7FFF_FFFF_FFFF_FFFF, 0x7FFF_FFFF_FFFF_FFFF)]
    fn usize_as_i64_trivial_x64(#[case] input: usize, #[case] expected: i64) {
        assert_eq!(usize_as_i64(input), expected);
    }

    // tests `clippy::cast_possible_wrap`
    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(0x8000_0000_0000_0000, -0x8000_0000_0000_0000)]
    #[case(0x8000_0000_0000_0001, -0x7FFF_FFFF_FFFF_FFFF)]
    #[case(0x8000_0000_0000_0003, -0x7FFF_FFFF_FFFF_FFFD)]
    #[case(0x8000_0000_0000_000F, -0x7FFF_FFFF_FFFF_FFF1)]
    #[case(0x8000_0000_0000_00FF, -0x7FFF_FFFF_FFFF_FF01)]
    #[case(0x8000_0000_0000_FFFF, -0x7FFF_FFFF_FFFF_0001)]
    #[case(0x8000_0000_FFFF_FFFF, -0x7FFF_FFFF_0000_0001)]
    #[case(0x8FFF_FFFF_FFFF_FFFF, -0x7000_0000_0000_0001)]
    #[case(0xF000_0000_0000_0000, -0x1000_0000_0000_0000)]
    #[case(0xFFFF_FFFF_0000_0000, -0x1_0000_0000)]
    #[case(0xFFFF_FFFF_FFFF_0000, -0x1_0000)]
    #[case(0xFFFF_FFFF_FFFF_FFFF, -0x1)]
    fn usize_as_i64_wrapping_x64(#[case] input: usize, #[case] expected: i64) {
        assert_eq!(usize_as_i64(input), expected);
    }

    #[rstest]
    #[case(0x0, 0x0)]
    #[case(0x1, 0x1)]
    #[case(0x3, 0x3)]
    #[case(0xF, 0xF)]
    #[case(0xFF, 0xFF)]
    #[case(0xFFFF, 0xFFFF)]
    #[case(0xFFFF_FFFF, 0xFFFF_FFFF)]
    fn i64_as_usize_trivial(#[case] input: i64, #[case] expected: usize) {
        assert_eq!(i64_as_usize(input), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(0x1_0000_0000, 0x1_0000_0000)]
    #[case(0x7FFF_FFFF_FFFF_FFFF, 0x7FFF_FFFF_FFFF_FFFF)]
    fn i64_as_usize_trivial_x64(#[case] input: i64, #[case] expected: usize) {
        assert_eq!(i64_as_usize(input), expected);
    }

    // tests `clippy::cast_sign_loss`
    #[cfg(target_pointer_width = "32")]
    #[rstest]
    #[case(-0x8000_0000_0000_0000, 0x0000_0000)]
    #[case(-0x7FFF_FFFF_FFFF_FFFF, 0x0000_0001)]
    #[case(-0x7FFF_FFFF_FFFF_FFFD, 0x0000_0003)]
    #[case(-0x7FFF_FFFF_FFFF_FFF1, 0x0000_000F)]
    #[case(-0x7FFF_FFFF_FFFF_FF01, 0x0000_00FF)]
    #[case(-0x7FFF_FFFF_FFFF_0001, 0x0000_FFFF)]
    #[case(-0x7FFF_FFFF_0000_0001, 0xFFFF_FFFF)]
    #[case(-0x1, 0xFFFF_FFFF)]
    fn i64_as_usize_sign_losing_x86(#[case] input: i64, #[case] expected: usize) {
        assert_eq!(i64_as_usize(input), expected);
    }

    // tests `clippy::cast_sign_loss`
    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(-0x8000_0000_0000_0000, 0x8000_0000_0000_0000)]
    #[case(-0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0001)]
    #[case(-0x7FFF_FFFF_FFFF_FFFD, 0x8000_0000_0000_0003)]
    #[case(-0x7FFF_FFFF_FFFF_FFF1, 0x8000_0000_0000_000F)]
    #[case(-0x7FFF_FFFF_FFFF_FF01, 0x8000_0000_0000_00FF)]
    #[case(-0x7FFF_FFFF_FFFF_0001, 0x8000_0000_0000_FFFF)]
    #[case(-0x7FFF_FFFF_0000_0001, 0x8000_0000_FFFF_FFFF)]
    #[case(-0x7000_0000_0000_0001, 0x8FFF_FFFF_FFFF_FFFF)]
    #[case(-0x1000_0000_0000_0000, 0xF000_0000_0000_0000)]
    #[case(-0x1_0000_0000, 0xFFFF_FFFF_0000_0000)]
    #[case(-0x1_0000, 0xFFFF_FFFF_FFFF_0000)]
    #[case(-0x1, 0xFFFF_FFFF_FFFF_FFFF)]
    fn i64_as_usize_sign_losing_x64(#[case] input: i64, #[case] expected: usize) {
        assert_eq!(i64_as_usize(input), expected);
    }

    // tests `clippy::cast_possible_truncation`
    #[cfg(target_pointer_width = "32")]
    #[rstest]
    #[case(0x1_0000_0000, 0x0)]
    #[case(0x1_0000_0001, 0x1)]
    #[case(0x1_0000_000C, 0xC)]
    #[case(0x1_0000_000F, 0xF)]
    #[case(0x1_0000_00FF, 0xFF)]
    #[case(0x1_0000_FFFF, 0xFFFF)]
    #[case(0x1_FFFF_FFFF, 0xFFFF_FFFF)]
    #[case(0x7FFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF)]
    fn i64_as_usize_truncating_x86(#[case] input: i64, #[case] expected: usize) {
        assert_eq!(i64_as_usize(input), expected);
    }

    #[rstest]
    #[case(0x0, 0x0)]
    #[case(0x1, 0x1)]
    #[case(0x3, 0x3)]
    #[case(0xF, 0xF)]
    #[case(0xFF, 0xFF)]
    #[case(0xFFFF, 0xFFFF)]
    #[case(0xFFFF_FFFF, 0xFFFF_FFFF)]
    fn usize_as_i64_as_usize_trivial(#[case] input: usize, #[case] expected: usize) {
        assert_eq!(i64_as_usize(usize_as_i64(input)), expected);
    }

    #[rstest]
    #[case(0x0, 0x0)]
    #[case(0x1, 0x1)]
    #[case(0x3, 0x3)]
    #[case(0xF, 0xF)]
    #[case(0xFF, 0xFF)]
    #[case(0xFFFF, 0xFFFF)]
    #[case(0xFFFF_FFFF, 0xFFFF_FFFF)]
    fn i64_as_usize_as_i64_trivial(#[case] input: i64, #[case] expected: i64) {
        assert_eq!(usize_as_i64(i64_as_usize(input)), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(0x1_0000_0000, 0x1_0000_0000)]
    #[case(0x7FFF_FFFF_FFFF_FFFF, 0x7FFF_FFFF_FFFF_FFFF)]
    fn usize_as_i64_as_usize_trivial_x64(#[case] input: usize, #[case] expected: usize) {
        assert_eq!(i64_as_usize(usize_as_i64(input)), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(0x1_0000_0000, 0x1_0000_0000)]
    #[case(0x7FFF_FFFF_FFFF_FFFF, 0x7FFF_FFFF_FFFF_FFFF)]
    fn i64_as_usize_as_i64_trivial_x64(#[case] input: i64, #[case] expected: i64) {
        assert_eq!(usize_as_i64(i64_as_usize(input)), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(0x8000_0000_0000_0000, 0x8000_0000_0000_0000)]
    #[case(0x8000_0000_0000_0001, 0x8000_0000_0000_0001)]
    #[case(0x8000_0000_0000_0003, 0x8000_0000_0000_0003)]
    #[case(0x8000_0000_0000_000F, 0x8000_0000_0000_000F)]
    #[case(0x8000_0000_0000_00FF, 0x8000_0000_0000_00FF)]
    #[case(0x8000_0000_0000_FFFF, 0x8000_0000_0000_FFFF)]
    #[case(0x8000_0000_FFFF_FFFF, 0x8000_0000_FFFF_FFFF)]
    #[case(0x8FFF_FFFF_FFFF_FFFF, 0x8FFF_FFFF_FFFF_FFFF)]
    #[case(0xF000_0000_0000_0000, 0xF000_0000_0000_0000)]
    #[case(0xFFFF_FFFF_0000_0000, 0xFFFF_FFFF_0000_0000)]
    #[case(0xFFFF_FFFF_FFFF_0000, 0xFFFF_FFFF_FFFF_0000)]
    #[case(0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF)]
    fn usize_as_i64_as_usize_nontrivial_x64(#[case] input: usize, #[case] expected: usize) {
        assert_eq!(i64_as_usize(usize_as_i64(input)), expected);
    }

    // tests `clippy::cast_possible_truncation`
    #[cfg(target_pointer_width = "32")]
    #[rstest]
    #[case(0x1_0000_0000, 0x0)]
    #[case(0x1_0000_0001, 0x1)]
    #[case(0x1_0000_000C, 0xC)]
    #[case(0x1_0000_000F, 0xF)]
    #[case(0x1_0000_00FF, 0xFF)]
    #[case(0x1_0000_FFFF, 0xFFFF)]
    #[case(0x1_FFFF_FFFF, 0xFFFF_FFFF)]
    #[case(0x7FFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF)]
    fn i64_as_usize_as_i64_truncating_x86(#[case] input: i64, #[case] expected: i64) {
        assert_eq!(usize_as_i64(i64_as_usize(input)), expected);
    }

    // tests `clippy::cast_sign_loss`
    #[cfg(target_pointer_width = "32")]
    #[rstest]
    #[case(-0x8000_0000_0000_0000, 0x0000_0000)]
    #[case(-0x7FFF_FFFF_FFFF_FFFF, 0x0000_0001)]
    #[case(-0x7FFF_FFFF_FFFF_FFFD, 0x0000_0003)]
    #[case(-0x7FFF_FFFF_FFFF_FFF1, 0x0000_000F)]
    #[case(-0x7FFF_FFFF_FFFF_FF01, 0x0000_00FF)]
    #[case(-0x7FFF_FFFF_FFFF_0001, 0x0000_FFFF)]
    #[case(-0x7FFF_FFFF_0000_0001, 0xFFFF_FFFF)]
    #[case(-0x1, 0xFFFF_FFFF)]
    fn i64_as_usize_as_i64_sign_losing_x86(#[case] input: i64, #[case] expected: i64) {
        assert_eq!(usize_as_i64(i64_as_usize(input)), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(-0x8000_0000_0000_0000, -0x8000_0000_0000_0000)]
    #[case(-0x7FFF_FFFF_FFFF_FFFF, -0x7FFF_FFFF_FFFF_FFFF)]
    #[case(-0x7FFF_FFFF_FFFF_FFFD, -0x7FFF_FFFF_FFFF_FFFD)]
    #[case(-0x7FFF_FFFF_FFFF_FFF1, -0x7FFF_FFFF_FFFF_FFF1)]
    #[case(-0x7FFF_FFFF_FFFF_FF01, -0x7FFF_FFFF_FFFF_FF01)]
    #[case(-0x7FFF_FFFF_FFFF_0001, -0x7FFF_FFFF_FFFF_0001)]
    #[case(-0x7FFF_FFFF_0000_0001, -0x7FFF_FFFF_0000_0001)]
    #[case(-0x7000_0000_0000_0001, -0x7000_0000_0000_0001)]
    #[case(-0x1000_0000_0000_0000, -0x1000_0000_0000_0000)]
    #[case(-0x1_0000_0000, -0x1_0000_0000)]
    #[case(-0x1_0000, -0x1_0000)]
    #[case(-0x1, -0x1)]
    fn i64_as_usize_as_i64_nontrivial_x64(#[case] input: i64, #[case] expected: i64) {
        assert_eq!(usize_as_i64(i64_as_usize(input)), expected);
    }

    #[rstest]
    #[case(0x0, 0x0)]
    #[case(0x1, 0x1)]
    #[case(0x3, 0x3)]
    #[case(0xF, 0xF)]
    #[case(0xFF, 0xFF)]
    fn usize_as_u8_trivial(#[case] input: usize, #[case] expected: u8) {
        assert_eq!(usize_as_u8(input), expected);
    }

    // tests `clippy::cast_possible_truncation`
    #[rstest]
    #[case(0x100, 0x0)]
    #[case(0x101, 0x1)]
    #[case(0x103, 0x3)]
    #[case(0x10F, 0xF)]
    #[case(0x1FF, 0xFF)]
    fn usize_as_u8_truncating(#[case] input: usize, #[case] expected: u8) {
        assert_eq!(usize_as_u8(input), expected);
    }

    #[rstest]
    #[case(0x0, hexf64!("0x0.p0"))]
    #[case(0x1, hexf64!("0x1.p0"))]
    #[case(0x3, hexf64!("0x3.p0"))]
    #[case(0xF, hexf64!("0xF.p0"))]
    #[case(0xFF, hexf64!("0xFF.p0"))]
    #[case(0xFFFF, hexf64!("0xFFFF.p0"))]
    #[case(0xFFFF_FFFF, hexf64!("0xFFFF_FFFF.p0"))]
    fn usize_as_f64_trivial(#[case] input: usize, #[case] expected: f64) {
        let l = usize_as_f64(input);
        assert!((l - expected).abs() < f64::EPSILON, "l={l}\nr={expected}",);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(0x1_0000_0000, hexf64!("0x1_0000_0000.p0"))]
    #[case(0x20_0000_0000_0000, hexf64!("0x20_0000_0000_0000.p0"))]
    fn usize_as_f64_trivial_x64(#[case] input: usize, #[case] expected: f64) {
        let l = usize_as_f64(input);
        assert!((l - expected).abs() < f64::EPSILON, "l={l}\nr={expected}",);
    }

    // tests `clippy::cast_precision_loss`
    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(0x20_0000_0000_0001, hexf64!("0x20_0000_0000_0000.p0"))]
    #[case(0x20_0000_0000_0003, hexf64!("0x20_0000_0000_0004.p0"))]
    #[case(0x20_0000_0000_000C, hexf64!("0x20_0000_0000_000C.p0"))]
    #[case(0x20_0000_0000_000F, hexf64!("0x20_0000_0000_0010.p0"))]
    #[case(0x20_0000_0000_00FF, hexf64!("0x20_0000_0000_0100.p0"))]
    #[case(0x20_0000_0000_FFFF, hexf64!("0x20_0000_0001_0000.p0"))]
    #[case(0x20_0000_FFFF_FFFF, hexf64!("0x20_0001_0000_0000.p0"))]
    #[case(0x2F_FFFF_FFFF_FFFF, hexf64!("0x30_0000_0000_0000.p0"))]
    #[case(0xFFFF_FFFF_FFFF_FFFF, hexf64!("0x8000_0000_0000_0000.p1"))]
    fn usize_as_f64_precision_losing_x64(#[case] input: usize, #[case] expected: f64) {
        let l = usize_as_f64(input);
        assert!((l - expected).abs() < f64::EPSILON, "l={l}\nr={expected}",);
    }

    #[rstest]
    #[case(hexf64!("0x0.p0"), 0x0)]
    #[case(hexf64!("-0x0.p0"), -0x0)]
    #[case(hexf64!("0x1.p0"), 0x1)]
    #[case(hexf64!("-0x1.p0"), -0x1)]
    #[case(hexf64!("0x3.p0"), 0x3)]
    #[case(hexf64!("-0x3.p0"), -0x3)]
    #[case(hexf64!("0xF.p0"), 0xF)]
    #[case(hexf64!("-0xF.p0"), -0xF)]
    #[case(hexf64!("0xFF.p0"), 0xFF)]
    #[case(hexf64!("-0xFF.p0"), -0xFF)]
    #[case(hexf64!("0xFFFF.p0"), 0xFFFF)]
    #[case(hexf64!("-0xFFFF.p0"), -0xFFFF)]
    #[case(hexf64!("0x7FFF_FFFF.p0"), 0x7FFF_FFFF)]
    #[case(hexf64!("-0x8000_0000.p0"), -0x8000_0000)]
    fn f64_as_isize_trivial(#[case] input: f64, #[case] expected: isize) {
        assert_eq!(f64_as_isize(input), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(hexf64!("0x8000_0000.p0"), 0x8000_0000)]
    #[case(hexf64!("-0x8000_0001.p0"), -0x8000_0001)]
    #[case(
        // 0x7FFF_FFFF_FFFF_FFFF.p0 cannot be exactly represented in f64, so rounding up
        hexf64!("0x8000_0000_0000_0000.p0"),
        0x7FFF_FFFF_FFFF_FFFF
    )]
    #[case(hexf64!("-0x8000_0000_0000_0000.p0"), -0x8000_0000_0000_0000)]
    fn f64_as_isize_trivial_x64(#[case] input: f64, #[case] expected: isize) {
        assert_eq!(f64_as_isize(input), expected);
    }

    // tests `clippy::cast_possible_truncation`
    #[rstest]
    #[case(hexf64!("0x0.FFFF_FFFF_FFFF_F8p0"), 0x0)]
    #[case(hexf64!("-0x0.FFFF_FFFF_FFFF_F8p0"), 0x0)]
    #[case(hexf64!("0x1.0000_0000_0000_1p0"), 0x1)]
    #[case(hexf64!("-0x1.0000_0000_0000_1p0"), -0x1)]
    #[case(hexf64!("0x0.0000_0000_0000_1p-1022"), 0x0)] // {min,max} x {pos,neg} subnormal numbers
    #[case(hexf64!("-0x0.0000_0000_0000_1p-1022"), 0x0)]
    #[case(hexf64!("0x0.FFFF_FFFF_FFFF_Fp-1022"), 0x0)]
    #[case(hexf64!("-0x0.FFFF_FFFF_FFFF_Fp-1022"), 0x0)]
    fn f64_as_isize_truncating(#[case] input: f64, #[case] expected: isize) {
        assert_eq!(f64_as_isize(input), expected);
    }

    // tests `clippy::cast_possible_truncation`
    #[cfg(target_pointer_width = "32")]
    #[rstest]
    #[case(hexf64!("0x8000_0000.p0"), 0x7FFF_FFFF)]
    #[case(hexf64!("-0x8000_0000.p0"), -0x8000_0000)]
    #[case(hexf64!("0x1.FFFF_FFFF_FFFF_Fp+1_023"), 0x7FFF_FFFF)]
    #[case(hexf64!("-0x1.FFFF_FFFF_FFFF_Fp+1_023"), -0x8000_0000)]
    fn f64_as_isize_truncating_x86(#[case] input: f64, #[case] expected: isize) {
        assert_eq!(f64_as_isize(input), expected);
    }

    // tests `clippy::cast_possible_truncation`
    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(hexf64!("0x8000_0000_0000_0800.p0"), 0x7FFF_FFFF_FFFF_FFFF)]
    #[case(hexf64!("-0x8000_0000_0000_0800.p0"), -0x8000_0000_0000_0000)]
    #[case(hexf64!("0x1.FFFF_FFFF_FFFF_Fp+1_023"), 0x7FFF_FFFF_FFFF_FFFF)]
    #[case(hexf64!("-0x1.FFFF_FFFF_FFFF_Fp+1_023"), -0x8000_0000_0000_0000)]
    fn f64_as_isize_truncating_x64(#[case] input: f64, #[case] expected: isize) {
        assert_eq!(f64_as_isize(input), expected);
    }

    #[rstest]
    #[case(f64::NAN, 0x0)]
    #[case(-f64::NAN, 0x0)]
    fn f64_as_isize_nontrivial(#[case] input: f64, #[case] expected: isize) {
        assert_eq!(f64_as_isize(input), expected);
    }

    #[cfg(target_pointer_width = "32")]
    #[rstest]
    #[case(f64::INFINITY, 0x7FFF_FFFF)]
    #[case(f64::NEG_INFINITY, -0x8000_0000)]
    fn f64_as_isize_nontrivial_x86(#[case] input: f64, #[case] expected: isize) {
        assert_eq!(f64_as_isize(input), expected);
    }

    #[cfg(target_pointer_width = "64")]
    #[rstest]
    #[case(f64::INFINITY, 0x7FFF_FFFF_FFFF_FFFF)]
    #[case(f64::NEG_INFINITY, -0x8000_0000_0000_0000)]
    fn f64_as_isize_nontrivial_x64(#[case] input: f64, #[case] expected: isize) {
        assert_eq!(f64_as_isize(input), expected);
    }

    #[rstest]
    #[case(0x0, 0x0)]
    #[case(0x1, 0x1)]
    #[case(0x3, 0x3)]
    #[case(0xF, 0xF)]
    #[case(0xFF, 0xFF)]
    #[case(0xFFFF, 0xFFFF)]
    fn i64_as_u16_trivial(#[case] input: i64, #[case] expected: u16) {
        assert_eq!(i64_as_u16(input), expected);
    }

    // tests `clippy::cast_sign_loss`
    #[rstest]
    #[case(-0x8000_0000_0000_0000, 0x0000)]
    #[case(-0x7FFF_FFFF_FFFF_FFFF, 0x0001)]
    #[case(-0x7FFF_FFFF_FFFF_FFFD, 0x0003)]
    #[case(-0x7FFF_FFFF_FFFF_FFF1, 0x000F)]
    #[case(-0x7FFF_FFFF_FFFF_FF01, 0x00FF)]
    #[case(-0x7FFF_FFFF_FFFF_0001, 0xFFFF)]
    #[case(-0x1, 0xFFFF)]
    fn i64_as_u16_sign_losing(#[case] input: i64, #[case] expected: u16) {
        assert_eq!(i64_as_u16(input), expected);
    }

    // tests `clippy::cast_possible_truncation`
    #[rstest]
    #[case(0x1_0000, 0x0)]
    #[case(0x1_0001, 0x1)]
    #[case(0x1_000C, 0xC)]
    #[case(0x1_000F, 0xF)]
    #[case(0x1_00FF, 0xFF)]
    #[case(0x1_FFFF, 0xFFFF)]
    #[case(0x7FFF_FFFF_FFFF_FFFF, 0xFFFF)]
    fn i64_as_u16_truncating(#[case] input: i64, #[case] expected: u16) {
        assert_eq!(i64_as_u16(input), expected);
    }

    #[rstest]
    #[case(hexf64!("0x0.p0"), 0x0)]
    #[case(hexf64!("-0x0.p0"), 0x0)]
    #[case(hexf64!("0x1.p0"), 0x1)]
    #[case(hexf64!("0x3.p0"), 0x3)]
    #[case(hexf64!("0xF.p0"), 0xF)]
    #[case(hexf64!("0xFF.p0"), 0xFF)]
    #[case(hexf64!("0xFFFF.p0"), 0xFFFF)]
    #[case(hexf64!("0xFFFF_FFFF.p0"), 0xFFFF_FFFF)]
    fn f64_as_u32_trivial(#[case] input: f64, #[case] expected: u32) {
        assert_eq!(f64_as_u32(input), expected);
    }

    #[rstest]
    #[case(f64::NAN, 0x0)]
    #[case(-f64::NAN, 0x0)]
    #[case(f64::INFINITY, 0xFFFF_FFFF)]
    #[case(f64::NEG_INFINITY, 0x0)]
    fn f64_as_u32_nontrivial(#[case] input: f64, #[case] expected: u32) {
        assert_eq!(f64_as_u32(input), expected);
    }

    #[rstest]
    #[case(hexf64!("0x0.FFFF_FFFF_FFFF_F8p0"), 0x0)]
    #[case(hexf64!("0x1.0000_0000_0000_1p0"), 0x1)]
    #[case(hexf64!("0x0.0000_0000_0000_1p-1022"), 0x0)] // {min,max} pos subnormal numbers
    #[case(hexf64!("0x0.FFFF_FFFF_FFFF_Fp-1022"), 0x0)]
    #[case(hexf64!("0x1_0000_0000.p0"), 0xFFFF_FFFF)]
    #[case(hexf64!("0x1.FFFF_FFFF_FFFF_Fp+1_023"), 0xFFFF_FFFF)]
    fn f64_as_u32_truncating(#[case] input: f64, #[case] expected: u32) {
        assert_eq!(f64_as_u32(input), expected);
    }

    #[rstest]
    #[case(hexf64!("-0x0.0000_0000_0000_1p-1022"), 0x0)]
    #[case(hexf64!("-0x1.FFFF_FFFF_FFFF_Fp+1_023"), 0x0)]
    fn f64_as_u32_sign_losing(#[case] input: f64, #[case] expected: u32) {
        assert_eq!(f64_as_u32(input), expected);
    }
}
