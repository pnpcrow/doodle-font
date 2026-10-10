//! Unicode mappings for both Windows BMP consumers and full-Unicode shapers.
//! https://learn.microsoft.com/en-us/typography/opentype/spec/cmap
use super::{check, B};
use crate::i18n::text as tr;
use std::collections::BTreeMap;

fn bmp(map: &BTreeMap<u32, u16>) -> Result<B, String> {
    let entries: Vec<_> = map
        .range(..0xffff)
        .map(|(&cp, &gid)| (cp as u16, gid))
        .collect();
    // Minimize encoded bytes: a delta segment costs 8, an indexed segment costs
    // 8 + 2 * its codepoint span (including holes). A greedy per-character writer
    // overflows format 4's uint16 length for ordinary CJK or color-layer fonts.
    let mut cost = vec![0i32; entries.len() + 1];
    let mut previous = vec![(0usize, false); entries.len() + 1];
    let mut indexed = (i32::MAX, 0);
    let mut delta = (i32::MAX, 0);
    for (i, &(cp, gid)) in entries.iter().enumerate() {
        let candidate = cost[i] + 8 - 2 * i32::from(cp);
        if candidate < indexed.0 {
            indexed = (candidate, i);
        }
        if i == 0
            || entries[i - 1].0 + 1 != cp
            || entries[i - 1].1.wrapping_add(1) != gid
            || cost[i] + 8 < delta.0
        {
            delta = (cost[i] + 8, i);
        }
        let indexed_cost = indexed.0 + 2 * (i32::from(cp) + 1);
        if delta.0 <= indexed_cost {
            cost[i + 1] = delta.0;
            previous[i + 1] = (delta.1, false);
        } else {
            cost[i + 1] = indexed_cost;
            previous[i + 1] = (indexed.1, true);
        }
    }
    let length = 16 + cost[entries.len()] as usize + 8; // header and terminal segment
    check(length <= u16::MAX as usize, tr("Windows 호환 문자 매핑 표가 너무 커요. 문자 모음별 분할 출력을 사용하거나 출력 범위를 줄여 주세요."))?;
    let mut segments = vec![];
    let mut end = entries.len();
    while end > 0 {
        let (start, indexed) = previous[end];
        segments.push((start, end, indexed));
        end = start;
    }
    segments.reverse();
    let count = segments.len() as u16 + 1;
    let power = 1u16 << count.ilog2();
    let mut b = B::default();
    b.u16(4);
    b.u16(length as u16);
    b.u16(0);
    b.u16(count * 2);
    b.u16(power * 2);
    b.u16(count.ilog2() as u16);
    b.u16((count - power) * 2);
    for &(_, end, _) in &segments {
        b.u16(entries[end - 1].0);
    }
    b.u16(0xffff);
    b.u16(0);
    for &(start, _, _) in &segments {
        b.u16(entries[start].0);
    }
    b.u16(0xffff);
    for &(start, _, indexed) in &segments {
        b.u16(if indexed {
            0
        } else {
            entries[start].1.wrapping_sub(entries[start].0)
        });
    }
    b.u16(map.get(&0xffff).copied().unwrap_or(0).wrapping_add(1));
    let mut glyphs = B::default();
    for (i, &(start, end, indexed)) in segments.iter().enumerate() {
        if indexed {
            b.u16((2 * (count as usize - i) + glyphs.0.len()) as u16);
            let mut entry = start;
            for cp in entries[start].0..=entries[end - 1].0 {
                let gid = if entries[entry].0 == cp {
                    let gid = entries[entry].1;
                    entry += 1;
                    gid
                } else {
                    0
                };
                glyphs.u16(gid);
            }
        } else {
            b.u16(0);
        }
    }
    b.u16(0);
    b.bytes(&glyphs.0);
    debug_assert_eq!(b.0.len(), length);
    Ok(b)
}

pub(super) fn table(
    map: &BTreeMap<u32, u16>,
    uvs: &BTreeMap<u32, Vec<(u32, u16)>>,
) -> Result<B, String> {
    let c4 = bmp(map)?;
    let mut c12 = B::default();
    c12.u16(12);
    c12.u16(0);
    c12.u32((16 + map.len() * 12) as u32);
    c12.u32(0);
    c12.u32(map.len() as u32);
    for (cp, g) in map {
        c12.u32(*cp);
        c12.u32(*cp);
        c12.u32(*g as u32);
    }
    let mut c14 = B::default();
    if !uvs.is_empty() {
        c14.u16(14);
        c14.u32(0);
        c14.u32(uvs.len() as u32);
        let mut data = B::default();
        for (v, items) in uvs {
            c14.bytes(&v.to_be_bytes()[1..]);
            c14.u32(0);
            c14.u32((10 + uvs.len() * 11 + data.0.len()) as u32);
            data.u32(items.len() as u32);
            for (cp, g) in items {
                data.bytes(&cp.to_be_bytes()[1..]);
                data.u16(*g);
            }
        }
        c14.bytes(&data.0);
        let len = c14.0.len();
        c14.put(2, len as u32);
    }
    let n = if uvs.is_empty() { 4 } else { 5 };
    let c4_offset = 4 + n * 8;
    let c12_offset = c4_offset + c4.0.len() as u32;
    let c14_offset = c12_offset + c12.0.len() as u32;
    let mut b = B::default();
    b.u16(0);
    b.u16(n as u16);
    // Sorted by platform/encoding. Share identical subtable bytes between records.
    for (platform, encoding, offset) in [(0, 3, c4_offset), (0, 4, c12_offset)] {
        b.u16(platform);
        b.u16(encoding);
        b.u32(offset);
    }
    if !uvs.is_empty() {
        b.u16(0);
        b.u16(5);
        b.u32(c14_offset);
    }
    for (encoding, offset) in [(1, c4_offset), (10, c12_offset)] {
        b.u16(3);
        b.u16(encoding);
        b.u32(offset);
    }
    b.bytes(&c4.0);
    b.bytes(&c12.0);
    b.bytes(&c14.0);
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(map: &BTreeMap<u32, u16>) {
        let encoded = bmp(map).unwrap();
        assert_eq!(
            u16::from_be_bytes([encoded.0[2], encoded.0[3]]) as usize,
            encoded.0.len()
        );
        let parsed = ttf_parser::cmap::Subtable4::parse(&encoded.0).unwrap();
        let read = |at: usize| u16::from_be_bytes([encoded.0[at], encoded.0[at + 1]]);
        let segments = read(6) as usize / 2;
        // Include holes, the terminal segment, and the boundary to supplementary Unicode.
        for cp in 0..=0x10000 {
            let expected = if cp <= 0xffff {
                map.get(&cp).copied().unwrap_or(0)
            } else {
                0
            };
            let actual = (0..segments)
                .find(|&i| u32::from(read(14 + i * 2)) >= cp)
                .map_or(0, |i| {
                    let start = u32::from(read(16 + segments * 2 + i * 2));
                    if cp < start {
                        return 0;
                    }
                    let delta = read(16 + segments * 4 + i * 2);
                    let offset_pos = 16 + segments * 6 + i * 2;
                    let offset = read(offset_pos) as usize;
                    if offset == 0 {
                        return (cp as u16).wrapping_add(delta);
                    }
                    let glyph = read(offset_pos + offset + (cp - start) as usize * 2);
                    if glyph == 0 {
                        0
                    } else {
                        glyph.wrapping_add(delta)
                    }
                });
            assert_eq!(actual, expected, "U+{cp:04X}");
            // ttf-parser 0.25.1 interprets indexed IDs >=32768 as negative and
            // returns None. Check those with the unsigned decoder above instead.
            if expected < 32768 {
                assert_eq!(
                    parsed.glyph_index(cp).map_or(0, |g| g.0),
                    expected,
                    "U+{cp:04X}"
                );
            }
        }
    }

    #[test]
    fn bmp_preserves_sparse_indexed_delta_and_terminal_mappings() {
        roundtrip(&BTreeMap::new());
        roundtrip(&BTreeMap::from([
            (0, 7),
            (65, 4),
            (66, 5),
            (68, 1),
            (70, 12),
            (0xfffe, 65534),
            (0xffff, 65533),
            (0x10000, 2),
        ]));
        // Many holes and unrelated glyph IDs exercise indexed segments and delta transitions.
        for seed in [1u32, 19, 871] {
            let mut map = BTreeMap::new();
            let mut state = seed;
            for cp in 0..12000 {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                if state & 3 != 0 {
                    map.insert(cp, ((state >> 16) as u16).max(1));
                }
            }
            roundtrip(&map);
        }
    }

    #[test]
    fn bmp_compacts_large_cjk_and_color_layer_glyph_ids_without_truncation() {
        let contiguous: BTreeMap<_, _> = (0..60000).map(|i| (i, i as u16 + 1)).collect();
        assert!(bmp(&contiguous).unwrap().0.len() < 40);
        roundtrip(&contiguous);
        let color: BTreeMap<_, _> = (0..16000)
            .map(|i| (0x4e00 + i, (i * 3 + 1) as u16))
            .collect();
        assert!(bmp(&color).unwrap().0.len() < 32100);
        roundtrip(&color);
    }

    #[test]
    fn unrepresentable_bmp_map_reports_split_instead_of_emitting_corrupt_length() {
        // Arbitrary non-consecutive glyph IDs cannot all fit in a uint16-sized format 4.
        let map: BTreeMap<_, _> = (0..60000).map(|i| (i, (i % 2 + 1) as u16)).collect();
        let error = match bmp(&map) {
            Ok(_) => panic!("oversized cmap accepted"),
            Err(e) => e,
        };
        assert!(error.contains("분할"));
    }
}
