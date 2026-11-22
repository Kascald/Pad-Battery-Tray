//! 아이콘 픽셀 위에 슬롯 번호를 찍는다. 16px에서도 읽히도록 3×5 비트맵 숫자에 어두운 외곽선을 두른다.

/// 3×5 숫자. 각 행의 하위 3비트가 왼쪽→오른쪽 픽셀이다.
const GLYPHS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

const FG: u32 = 0xFFFF_FFFF;
const OUTLINE: u32 = 0xFF10_1010;

fn lit(digit: usize, x: usize, y: usize) -> bool {
    GLYPHS[digit][y] & (0b100 >> x) != 0
}

/// `pixels`는 위→아래 순서의 `size`×`size` ARGB(비 premultiplied)다. 오른쪽 아래 모서리에 찍는다.
pub fn stamp(pixels: &mut [u32], size: usize, digit: u8) {
    assert_eq!(pixels.len(), size * size);
    let digit = usize::from(digit % 10);
    let s = (size / 16).max(1);
    let (w, h) = (5 * s, 7 * s);
    if size < w || size < h {
        return;
    }
    let (ox, oy) = (size - w, size - h);

    // 셀 단위(외곽선 포함 5×7)로 판정한 뒤 s배로 키운다
    for cy in 0..7 {
        for cx in 0..5 {
            let glyph = |gx: isize, gy: isize| {
                (0..3).contains(&gx) && (0..5).contains(&gy) && lit(digit, gx as usize, gy as usize)
            };
            let (gx, gy) = (cx as isize - 1, cy as isize - 1);
            let color = if glyph(gx, gy) {
                FG
            } else if (-1..=1).any(|dy| (-1..=1).any(|dx| glyph(gx + dx, gy + dy))) {
                OUTLINE
            } else {
                continue;
            };
            for py in 0..s {
                let row = (oy + cy * s + py) * size;
                pixels[row + ox + cx * s..row + ox + cx * s + s].fill(color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(size: usize, digit: u8) -> Vec<String> {
        let mut px = vec![0u32; size * size];
        stamp(&mut px, size, digit);
        px.chunks(size)
            .map(|row| {
                row.iter()
                    .map(|&p| match p {
                        FG => '#',
                        OUTLINE => 'o',
                        _ => '.',
                    })
                    .collect()
            })
            .collect()
    }

    fn corner(size: usize, digit: u8, w: usize) -> Vec<String> {
        render(size, digit).into_iter().skip(size - 7 * (size / 16).max(1)).map(|r| r[size - w..].to_string()).collect()
    }

    #[test]
    fn stamps_bottom_right_at_16px() {
        assert_eq!(corner(16, 2, 5), ["ooooo", "o###o", "ooo#o", "o###o", "o#ooo", "o###o", "ooooo"]);
        assert_eq!(corner(16, 4, 5), ["ooooo", "o#o#o", "o#o#o", "o###o", "ooo#o", "..o#o", "..ooo"]);
        assert!(render(16, 4)[..9].iter().all(|r| r.chars().all(|c| c == '.')));
        assert!(render(16, 4).iter().all(|r| r[..11].chars().all(|c| c == '.')));
    }

    #[test]
    fn scales_with_size() {
        let rows = corner(32, 4, 10);
        assert_eq!(rows.len(), 14);
        assert_eq!(rows[2], "oo##oo##oo");
        assert_eq!(rows[3], "oo##oo##oo");
    }

    #[test]
    fn leaves_other_pixels_untouched() {
        let size = 24;
        let mut px = vec![0x8012_3456u32; size * size];
        stamp(&mut px, size, 2);
        assert_eq!(px[0], 0x8012_3456);
        assert_eq!(px[size * 10 + 5], 0x8012_3456);
    }
}
