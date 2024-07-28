#[must_use]
pub const fn rgb_to_hex(rgb: [u8; 3]) -> u32 {
    ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | (rgb[2] as u32)
}

#[must_use]
pub const fn hex_to_rgb(hex: u32) -> [u8; 3] {
    [
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    ]
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    #[rstest]
    #[case([0  , 0  , 0  ], 0x00_00_00)]
    #[case([255, 0  , 0  ], 0xFF_00_00)]
    #[case([0  , 255, 0  ], 0x00_FF_00)]
    #[case([255, 255, 0  ], 0xFF_FF_00)]
    #[case([0  , 0  , 255], 0x00_00_FF)]
    #[case([255, 0  , 255], 0xFF_00_FF)]
    #[case([0  , 255, 255], 0x00_FF_FF)]
    #[case([255, 255, 255], 0xFF_FF_FF)]
    #[case([123, 45 , 67 ], 0x7B_2D_43)]
    #[case([89 , 101, 112], 0x59_65_70)]
    fn rgb_to_hex(#[case] input: [u8; 3], #[case] expected: u32) {
        assert_eq!(super::rgb_to_hex(input), expected);
    }

    #[rstest]
    #[case(0x00_00_00, [0  , 0  , 0  ])]
    #[case(0xFF_00_00, [255, 0  , 0  ])]
    #[case(0x00_FF_00, [0  , 255, 0  ])]
    #[case(0xFF_FF_00, [255, 255, 0  ])]
    #[case(0x00_00_FF, [0  , 0  , 255])]
    #[case(0xFF_00_FF, [255, 0  , 255])]
    #[case(0x00_FF_FF, [0  , 255, 255])]
    #[case(0xFF_FF_FF, [255, 255, 255])]
    #[case(0x7B_2D_43, [123, 45 , 67 ])]
    #[case(0x59_65_70, [89 , 101, 112])]
    fn hex_to_rgb(#[case] input: u32, #[case] expected: [u8; 3]) {
        assert_eq!(super::hex_to_rgb(input), expected);
    }

    #[rstest]
    #[case([0  , 0  , 0  ])]
    #[case([255, 0  , 0  ])]
    #[case([0  , 255, 0  ])]
    #[case([255, 255, 0  ])]
    #[case([0  , 0  , 255])]
    #[case([255, 0  , 255])]
    #[case([0  , 255, 255])]
    #[case([255, 255, 255])]
    #[case([123, 45 , 67 ])]
    #[case([89 , 101, 112])]
    fn rgb_to_hex_to_rgb(#[case] input: [u8; 3]) {
        assert_eq!(super::hex_to_rgb(super::rgb_to_hex(input)), input);
    }

    #[rstest]
    #[case(0x00_00_00)]
    #[case(0xFF_00_00)]
    #[case(0x00_FF_00)]
    #[case(0xFF_FF_00)]
    #[case(0x00_00_FF)]
    #[case(0xFF_00_FF)]
    #[case(0x00_FF_FF)]
    #[case(0xFF_FF_FF)]
    #[case(0x7B_2D_43)]
    #[case(0x59_65_70)]
    fn hex_to_rgb_to_hex(#[case] input: u32) {
        assert_eq!(super::rgb_to_hex(super::hex_to_rgb(input)), input);
    }
}
