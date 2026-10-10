use base64::{engine::general_purpose::STANDARD, Engine};
use doodle_core::{document::*, font, model::*};
use std::io::Read;

fn export(p: &Project, format: &str, mode: &str) -> (Vec<u8>, font::ExportResult) {
    let result = font::export(
        p,
        &font::ExportOptions {
            format: Some(format.into()),
            mode: Some(mode.into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.filename.ends_with(&format!(".{format}")));
    assert_eq!(result.reports[0].format, format);
    (STANDARD.decode(&result.data).unwrap(), result)
}
fn u16_at(b: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(b[offset..offset + 2].try_into().unwrap())
}
fn u32_at(b: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(b[offset..offset + 4].try_into().unwrap())
}
fn checksum(b: &[u8]) -> u32 {
    b.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

#[test]
fn desktop_cmaps_agree_for_bmp_supplementary_and_variation_characters() {
    let mut p = new_project();
    let mut hangul = new_glyph("한", "ko", "hangul", false);
    hangul.artwork = p.glyphs[0].artwork.clone();
    hangul.status = "edited".into();
    p.glyphs.push(hangul);
    for format in ["ttf", "otf"] {
        let (bytes, _) = export(&p, format, "color");
        let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
        let subtables = face.tables().cmap.unwrap().subtables;
        let bmp = subtables
            .into_iter()
            .find(|s| s.platform_id == ttf_parser::PlatformId::Windows && s.encoding_id == 1)
            .unwrap();
        let full = subtables
            .into_iter()
            .find(|s| s.platform_id == ttf_parser::PlatformId::Windows && s.encoding_id == 10)
            .unwrap();
        assert!(matches!(
            bmp.format,
            ttf_parser::cmap::Format::SegmentMappingToDeltaValues(_)
        ));
        assert!(matches!(
            full.format,
            ttf_parser::cmap::Format::SegmentedCoverage(_)
        ));
        for cp in 0..=0xffff {
            assert_eq!(
                bmp.glyph_index(cp).map_or(0, |g| g.0),
                full.glyph_index(cp).map_or(0, |g| g.0),
                "{format}: U+{cp:04X}"
            );
        }
        for c in ['A', '한', '😊'] {
            assert!(full.glyph_index(c as u32).is_some());
        }
        assert!(face.glyph_variation_index('❤', '\u{fe0f}').is_some());
    }
}

#[cfg(windows)]
#[test]
fn windows_loads_saved_ttf_and_otf_including_blank_and_supplementary_only_fonts() {
    use windows_sys::Win32::Graphics::Gdi::{
        AddFontResourceExW, RemoveFontResourceExW, FR_PRIVATE,
    };
    let mut fixtures = vec![new_project()];
    for text in [" ", "😊"] {
        let mut p = new_project();
        p.glyphs.clear();
        let mut g = new_glyph(text, "default", "custom", false);
        g.status = "blank".into();
        p.glyphs.push(g);
        fixtures.push(p);
    }
    for (i, p) in fixtures.iter().enumerate() {
        for format in ["ttf", "otf"] {
            for mode in ["mono", "color", "bitmap"] {
                // Export also exercises the process-private Windows memory font loader.
                let (bytes, _) = export(p, format, mode);
                let path = std::env::temp_dir().join(format!("두들-글꼴-검증-{}.{format}", uid()));
                std::fs::write(&path, bytes).unwrap();
                let wide: Vec<u16> = path
                    .as_os_str()
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                // Only this test process can see the temporary font. Never install it globally.
                let loaded =
                    unsafe { AddFontResourceExW(wide.as_ptr(), FR_PRIVATE, std::ptr::null()) };
                if loaded > 0 {
                    assert_ne!(
                        unsafe {
                            RemoveFontResourceExW(wide.as_ptr(), FR_PRIVATE, std::ptr::null())
                        },
                        0
                    );
                }
                std::fs::remove_file(path).unwrap();
                assert!(loaded > 0, "Windows rejected fixture {i}/{format}/{mode}");
            }
        }
    }
}
fn base128(b: &[u8], offset: &mut usize) -> usize {
    let mut n = 0usize;
    for _ in 0..5 {
        let v = b[*offset];
        *offset += 1;
        assert!(n != 0 || v != 0x80);
        n = (n << 7) | (v & 127) as usize;
        if v & 128 == 0 {
            return n;
        }
    }
    panic!("invalid base128");
}

#[test]
fn web_containers_preserve_every_font_table_and_checksums() {
    let p = new_project();
    for mode in ["mono", "color", "bitmap"] {
        let (ttf, _) = export(&p, "ttf", mode);
        let raw = ttf_parser::RawFace::parse(&ttf, 0).unwrap();
        let records: Vec<_> = raw.table_records.into_iter().collect();
        for format in ["woff", "woff2"] {
            let (bytes, _) = export(&p, format, mode);
            assert_eq!(
                &bytes[..4],
                if format == "woff" { b"wOFF" } else { b"wOF2" }
            );
            assert_eq!(u32_at(&bytes, 8) as usize, bytes.len());
            assert_eq!(u16_at(&bytes, 12) as usize, records.len());
            assert_eq!(u16_at(&bytes, 14), 0);
            assert_eq!(u32_at(&bytes, 16) as usize, ttf.len());
            if format == "woff" {
                for (i, record) in records.iter().enumerate() {
                    let entry = &bytes[44 + i * 20..44 + (i + 1) * 20];
                    assert_eq!(entry[..4], record.tag.to_bytes());
                    let offset = u32_at(entry, 4) as usize;
                    let len = u32_at(entry, 8) as usize;
                    assert_eq!(offset % 4, 0);
                    assert_eq!(u32_at(entry, 12), record.length);
                    assert_eq!(u32_at(entry, 16), record.check_sum);
                    let data = &bytes[offset..offset + len];
                    let decoded = if len < record.length as usize {
                        let mut decoded = vec![];
                        flate2::read::ZlibDecoder::new(data)
                            .read_to_end(&mut decoded)
                            .unwrap();
                        decoded
                    } else {
                        data.to_vec()
                    };
                    assert_eq!(decoded, raw.table(record.tag).unwrap());
                }
            } else {
                let mut offset = 48;
                let mut lengths = vec![];
                for record in &records {
                    let flags = bytes[offset];
                    offset += 1;
                    let expected_tag = match &record.tag.to_bytes() {
                        b"cmap" => 0,
                        b"head" => 1,
                        b"hhea" => 2,
                        b"hmtx" => 3,
                        b"maxp" => 4,
                        b"name" => 5,
                        b"OS/2" => 6,
                        b"post" => 7,
                        b"glyf" => 10,
                        b"loca" => 11,
                        b"GSUB" => 28,
                        b"COLR" => 34,
                        b"CPAL" => 35,
                        b"sbix" => 37,
                        tag => panic!("unverified tag {tag:?}"),
                    };
                    assert_eq!(flags & 63, expected_tag);
                    assert_eq!(
                        flags >> 6,
                        if expected_tag == 10 || expected_tag == 11 {
                            3
                        } else {
                            0
                        }
                    );
                    let len = base128(&bytes, &mut offset);
                    assert_eq!(len, record.length as usize);
                    lengths.push(len);
                }
                let end = offset + u32_at(&bytes, 20) as usize;
                assert_eq!((end + 3) & !3, bytes.len());
                assert!(bytes[end..].iter().all(|b| *b == 0));
                let mut decoded = vec![];
                brotli::Decompressor::new(&bytes[offset..], 4096)
                    .read_to_end(&mut decoded)
                    .unwrap();
                assert_eq!(decoded.len(), lengths.iter().sum::<usize>());
                let mut offset = 0;
                for (record, len) in records.iter().zip(lengths) {
                    let mut data = decoded[offset..offset + len].to_vec();
                    offset += len;
                    let expected = raw.table(record.tag).unwrap();
                    if record.tag.to_bytes() == *b"head" {
                        assert_eq!(u16_at(&data, 16), u16_at(expected, 16) | (1 << 11));
                        data[8..12].copy_from_slice(&expected[8..12]);
                        data[16..18].copy_from_slice(&expected[16..18]);
                    }
                    assert_eq!(data, expected);
                }
            }
        }
    }
}

fn shaped(bytes: &[u8], text: &str, lang: &str) -> Vec<(u32, i32)> {
    let face = rustybuzz::Face::from_slice(bytes, 0).unwrap();
    let mut b = rustybuzz::UnicodeBuffer::new();
    b.push_str(text);
    b.guess_segment_properties();
    b.set_language(lang.parse().unwrap());
    let result = rustybuzz::shape(&face, &[], b);
    result
        .glyph_infos()
        .iter()
        .zip(result.glyph_positions())
        .map(|(g, p)| (g.glyph_id, p.x_advance))
        .collect()
}

#[test]
fn cff_outlines_metrics_sequences_locales_and_color_match_truetype() {
    let mut p = new_project();
    for (text, locale) in [
        ("骨", "ko"),
        ("骨", "ja"),
        ("fi", "default"),
        (" ", "default"),
    ] {
        let mut g = new_glyph(text, locale, "custom", text == "fi");
        g.artwork = p.glyphs[0].artwork.clone();
        g.artwork.advance = if locale == "ja" { 900. } else { 700. };
        g.status = if text == " " { "blank" } else { "edited" }.into();
        p.glyphs.push(g);
    }
    for mode in ["mono", "color", "bitmap"] {
        let (ttf, _) = export(&p, "ttf", mode);
        let (otf, report) = export(&p, "otf", mode);
        assert_eq!(&otf[..4], b"OTTO");
        assert_eq!(checksum(&otf), 0xB1B0AFBA);
        assert!(report.reports[0].tables.contains(&"CFF ".into()));
        assert!(!report.reports[0].tables.contains(&"glyf".into()));
        assert!(!report.reports[0].tables.contains(&"loca".into()));
        let original = ttf_parser::Face::parse(&ttf, 0).unwrap();
        let face = ttf_parser::Face::parse(&otf, 0).unwrap();
        assert!(face.tables().cff.is_some());
        assert_eq!(face.number_of_glyphs(), original.number_of_glyphs());
        for gid in 0..face.number_of_glyphs() {
            let gid = ttf_parser::GlyphId(gid);
            assert_eq!(
                face.glyph_bounding_box(gid),
                original.glyph_bounding_box(gid)
            );
            assert_eq!(face.glyph_hor_advance(gid), original.glyph_hor_advance(gid));
        }
        for lang in ["ko", "ja"] {
            assert_eq!(
                shaped(&otf, "A❤️😊 fi 骨", lang),
                shaped(&ttf, "A❤️😊 fi 骨", lang)
            );
        }
        assert_eq!(shaped(&otf, "fi", "en").len(), 1);
        assert_ne!(shaped(&otf, "骨", "ko"), shaped(&otf, "骨", "ja"));
        let a = doodle_core::render::text(&ttf, "A❤️😊 fi 骨", "ja", 128.).unwrap();
        let b = doodle_core::render::text(&otf, "A❤️😊 fi 骨", "ja", 128.).unwrap();
        assert_eq!(a.dimensions(), b.dimensions());
        let different = a
            .pixels()
            .zip(b.pixels())
            .filter(|(a, b)| a.0.iter().zip(b.0).any(|(a, b)| a.abs_diff(b) > 2))
            .count();
        assert!(
            different * 1000 < a.pixels().len(),
            "OTF rendering changed: {mode}/{different}"
        );
    }
}

#[test]
fn split_subset_exports_use_requested_format_and_keep_rights_checks() {
    let p = new_project();
    for format in ["ttf", "otf", "woff", "woff2"] {
        let options = font::ExportOptions {
            format: Some(format.into()),
            mode: Some("mono".into()),
            split: Some(true),
            ids: Some(vec![p.glyphs[0].id.clone()]),
            ..Default::default()
        };
        let result = font::export(&p, &options).unwrap();
        assert!(result.filename.ends_with(".zip"));
        assert_eq!(result.reports.len(), 1);
        assert_eq!(result.reports[0].authored_count, 1);
        let mut zip =
            zip::ZipArchive::new(std::io::Cursor::new(STANDARD.decode(result.data).unwrap()))
                .unwrap();
        let names: Vec<_> = zip.file_names().map(str::to_string).collect();
        assert!(names
            .iter()
            .any(|name| name.ends_with(&format!(".{format}"))));
        let mut report = String::new();
        zip.by_name("font-report.json")
            .unwrap()
            .read_to_string(&mut report)
            .unwrap();
        assert!(report.contains(&format!("\"format\": \"{format}\"")));
        let mut denied = p.clone();
        denied.glyphs[0]
            .template
            .as_mut()
            .unwrap()
            .permissions
            .embed = false;
        assert!(font::export(&denied, &options)
            .err()
            .unwrap()
            .contains("권한"));
    }
    assert!(font::export(
        &p,
        &font::ExportOptions {
            format: Some("not-a-font".into()),
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn cff_supports_the_full_glyph_id_range() {
    let mut p = new_project();
    p.glyphs.clear();
    for i in 0..65534u32 {
        let text = char::from_u32(0x10000 + i).unwrap().to_string();
        let mut g = new_glyph(&text, "default", "custom", false);
        g.status = "blank".into();
        g.artwork.advance = 500.;
        p.glyphs.push(g);
    }
    let (bytes, _) = export(&p, "otf", "mono");
    let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
    assert_eq!(face.number_of_glyphs(), 65535);
    assert_eq!(
        face.glyph_index(char::from_u32(0x10000 + 65533).unwrap()),
        Some(ttf_parser::GlyphId(65534))
    );
    assert_eq!(
        face.glyph_hor_advance(ttf_parser::GlyphId(65534)),
        Some(500)
    );
}

#[test]
fn oversized_cff_charstring_is_rejected_without_limiting_truetype() {
    let mut p = new_project();
    p.glyphs.truncate(1);
    p.glyphs[0].artwork.shapes.truncate(1);
    let shape = &mut p.glyphs[0].artwork.shapes[0];
    shape.kind = "outline".into();
    shape.contours = vec![(0..30000)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / 30000.;
            [500. + 400. * angle.cos(), 500. + 400. * angle.sin()]
        })
        .collect()];
    p.validate().unwrap();
    let error = font::export(
        &p,
        &font::ExportOptions {
            format: Some("otf".into()),
            mode: Some("mono".into()),
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("OTF 윤곽선이 너무 복잡"));
    let (ttf, _) = export(&p, "ttf", "mono");
    assert!(ttf_parser::Face::parse(&ttf, 0).is_ok());
}

#[test]
fn cff_rejects_out_of_range_moves_without_panicking() {
    let mut p = new_project();
    p.glyphs.truncate(1);
    let mut shape = p.glyphs[0].artwork.shapes[0].clone();
    shape.kind = "outline".into();
    shape.width = 4000.;
    shape.height = 1000.;
    shape.x = 4000.;
    shape.contours = vec![vec![[3900., 0.], [4000., 0.], [4000., 100.]]];
    let mut far = shape.clone();
    far.id = "far-outline".into();
    far.x = -4000.;
    far.contours = vec![vec![[-4000., 0.], [-3900., 0.], [-4000., 100.]]];
    p.glyphs[0].artwork.shapes = vec![shape, far];
    p.validate().unwrap();
    let error = font::export(
        &p,
        &font::ExportOptions {
            format: Some("otf".into()),
            mode: Some("mono".into()),
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("거리가 너무 커요"));
}
