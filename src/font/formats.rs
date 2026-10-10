//! Native CFF1, WOFF1 and WOFF2 writers. WOFF2 uses the standard null transforms
//! and streams the original tables through Brotli, avoiding a second raw-font copy.
//! Specs: Adobe TN 5176/5177 and https://www.w3.org/TR/WOFF2/ (also /WOFF/).
use super::{check, Outline, B};
use std::io::{Read, Write};

fn number(b: &mut B, v: i16) {
    let v = i32::from(v);
    match v {
        -107..=107 => b.0.push((v + 139) as u8),
        108..=1131 => b.bytes(&[((v - 108) / 256 + 247) as u8, ((v - 108) % 256) as u8]),
        -1131..=-108 => b.bytes(&[((-v - 108) / 256 + 251) as u8, ((-v - 108) % 256) as u8]),
        _ => {
            b.0.push(28);
            b.i16(v as i16);
        }
    }
}

pub(super) fn charstring(
    points: &[[i16; 2]],
    lengths: &[usize],
    width: u16,
) -> Result<Vec<u8>, String> {
    let mut b = B::default();
    // Both nominalWidthX and defaultWidthX are zero. The first operator consumes width.
    number(&mut b, width as i16);
    let mut prev = [0i16; 2];
    let mut start = 0;
    for &len in lengths {
        let contour = &points[start..start + len];
        start += len;
        // Reverse TrueType winding for PostScript, keeping counters and overlaps intact.
        let first = contour[len - 1];
        for (i, &pt) in contour
            .iter()
            .rev()
            .chain(std::iter::once(&first))
            .enumerate()
        {
            for axis in 0..2 {
                let delta = pt[axis].checked_sub(prev[axis]).ok_or(
                    "OTF 윤곽선 사이의 거리가 너무 커요. 그림의 크기나 위치를 줄여 주세요.",
                )?;
                number(&mut b, delta);
            }
            b.0.push(if i == 0 { 21 } else { 5 }); // rmoveto / rlineto
            prev = pt;
        }
    }
    b.0.push(14); // endchar (also valid for blank glyphs, with their explicit width)
    Ok(b.0)
}

fn size(v: usize) -> Result<u32, String> {
    u32::try_from(v).map_err(|_| "폰트가 너무 커요. 문자 모음별로 분할 출력해 주세요.".into())
}

fn index(items: &[&[u8]]) -> Result<B, String> {
    let mut b = B::default();
    b.u16(u16::try_from(items.len()).map_err(|_| "OTF 글리프 수가 너무 많아요.")?);
    if items.is_empty() {
        return Ok(b);
    }
    b.0.push(4); // Fixed-size offsets keep Top DICT relocation independent of its values.
    let mut off = 1usize;
    for item in items {
        b.u32(size(off)?);
        off += item.len();
    }
    b.u32(size(off)?);
    for item in items {
        b.bytes(item);
    }
    Ok(b)
}

fn dict(b: &mut B, values: &[i32], op: &[u8]) {
    for v in values {
        b.0.push(29);
        b.bytes(&v.to_be_bytes());
    }
    b.bytes(op);
}

pub(super) fn cff(name: &str, outlines: &[Outline], bounds: [i16; 4]) -> Result<B, String> {
    // CID identity mapping supports all 65,535 glyphs without exhausting custom string IDs.
    let n = outlines.len();
    let names = index(&[name.as_bytes()])?;
    let strings = index(&[b"Adobe", b"Identity", name.as_bytes()])?;
    let chars = index(
        &outlines
            .iter()
            .map(|o| o.data.as_slice())
            .collect::<Vec<_>>(),
    )?;
    let mut charset = B(vec![2]);
    if n > 1 {
        charset.u16(1);
        charset.u16((n - 2) as u16);
    }
    let mut select = B(vec![3]);
    select.u16(1);
    select.u16(0);
    select.0.push(0);
    select.u16(n as u16);
    let private = [139, 20, 139, 21]; // defaultWidthX=0, nominalWidthX=0
    let font_dict = |offset: usize| -> Result<B, String> {
        let mut d = B::default();
        dict(&mut d, &[393], &[12, 38]); // FontName
        dict(&mut d, &[private.len() as i32, size(offset)? as i32], &[18]);
        index(&[&d.0])
    };
    let top_dict = |charset: usize, chars: usize, fd: usize, select: usize| -> Result<B, String> {
        let mut d = B::default();
        dict(&mut d, &[391, 392, 0], &[12, 30]); // ROS must be first
        dict(&mut d, &[n as i32], &[12, 34]);
        dict(&mut d, &[2], &[12, 6]); // CharstringType
        dict(&mut d, &bounds.map(i32::from), &[5]);
        dict(&mut d, &[size(charset)? as i32], &[15]);
        dict(&mut d, &[size(chars)? as i32], &[17]);
        dict(&mut d, &[size(fd)? as i32], &[12, 36]);
        dict(&mut d, &[size(select)? as i32], &[12, 37]);
        index(&[&d.0])
    };
    let charset_off = 4 + names.0.len() + top_dict(0, 0, 0, 0)?.0.len() + strings.0.len() + 2;
    let select_off = charset_off + charset.0.len();
    let chars_off = select_off + select.0.len();
    let fd_off = chars_off + chars.0.len();
    let private_off = fd_off + font_dict(0)?.0.len();
    check(
        private_off <= i32::MAX as usize,
        "OTF가 너무 커요. 분할 출력해 주세요.",
    )?;
    let top = top_dict(charset_off, chars_off, fd_off, select_off)?;
    let fd = font_dict(private_off)?;
    let mut b = B(vec![1, 0, 4, 4]);
    b.bytes(&names.0);
    b.bytes(&top.0);
    b.bytes(&strings.0);
    b.u16(0); // Empty Global Subr INDEX
    b.bytes(&charset.0);
    b.bytes(&select.0);
    b.bytes(&chars.0);
    b.bytes(&fd.0);
    b.bytes(&private);
    Ok(b)
}

struct Table<'a> {
    tag: [u8; 4],
    checksum: u32,
    data: &'a [u8],
}
fn tables(sfnt: &[u8]) -> Result<Vec<Table<'_>>, String> {
    let face =
        ttf_parser::RawFace::parse(sfnt, 0).map_err(|e| format!("웹폰트 원본 오류: {e:?}"))?;
    face.table_records
        .into_iter()
        .map(|r| {
            let start = r.offset as usize;
            let end = start
                .checked_add(r.length as usize)
                .ok_or("웹폰트 크기 오류")?;
            let data = sfnt.get(start..end).ok_or("웹폰트 테이블 오류")?;
            Ok(Table {
                tag: r.tag.to_bytes(),
                checksum: r.check_sum,
                data,
            })
        })
        .collect()
}

pub(super) fn woff(sfnt: &[u8]) -> Result<Vec<u8>, String> {
    let tables = tables(sfnt)?;
    let mut b = B::default();
    b.bytes(b"wOFF");
    b.bytes(&sfnt[..4]);
    b.u32(0);
    b.u16(tables.len() as u16);
    b.u16(0);
    b.u32(size(sfnt.len())?);
    b.u16(1);
    b.u16(0);
    b.zeros(20);
    b.zeros(tables.len() * 20);
    for (i, table) in tables.iter().enumerate() {
        let mut compressor =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        compressor
            .write_all(table.data)
            .map_err(|e| e.to_string())?;
        let compressed = compressor.finish().map_err(|e| e.to_string())?;
        let data = if compressed.len() < table.data.len() {
            &compressed
        } else {
            table.data
        };
        let pos = 44 + i * 20;
        b.0[pos..pos + 4].copy_from_slice(&table.tag);
        b.put(pos + 4, size(b.0.len())?);
        b.put(pos + 8, size(data.len())?);
        b.put(pos + 12, size(table.data.len())?);
        b.put(pos + 16, table.checksum);
        b.bytes(data);
        while b.0.len() % 4 != 0 {
            b.0.push(0);
        }
    }
    b.put(8, size(b.0.len())?);
    Ok(b.0)
}

fn base128(b: &mut B, v: u32) {
    let groups = ((32 - v.leading_zeros()).max(1)).div_ceil(7);
    for group in (0..groups).rev() {
        b.0.push(((v >> (group * 7)) & 127) as u8 | if group > 0 { 128 } else { 0 });
    }
}

pub(super) fn woff2(sfnt: &[u8]) -> Result<Vec<u8>, String> {
    const TAGS: &[u8] = b"cmapheadhheahmtxmaxpnameOS/2postcvt fpgmglyflocaprepCFF VORGEBDTEBLCgasphdmxkernLTSHPCLTVDMXvheavmtxBASEGDEFGPOSGSUBEBSCJSTFMATHCBDTCBLCCOLRCPALSVG sbixacntavarbdatblocbslncvarfdscfeatfmtxfvargvarhstyjustlcarmortmorxopbdproptrakZapfSilfGlatGlocFeatSill";
    let tables = tables(sfnt)?;
    let mut b = B::default();
    b.bytes(b"wOF2");
    b.bytes(&sfnt[..4]);
    b.u32(0);
    b.u16(tables.len() as u16);
    b.u16(0);
    b.u32(size(sfnt.len())?);
    b.u32(0);
    b.u16(1);
    b.u16(0);
    b.zeros(20);
    for table in &tables {
        let tag = TAGS
            .as_chunks::<4>()
            .0
            .iter()
            .position(|tag| *tag == table.tag)
            .unwrap_or(63) as u8;
        // Null transform is version 3 for glyf/loca, version 0 for all other tables.
        b.0.push(
            tag | if table.tag == *b"glyf" || table.tag == *b"loca" {
                0xc0
            } else {
                0
            },
        );
        if tag == 63 {
            b.bytes(&table.tag);
        }
        base128(&mut b, size(table.data.len())?);
    }
    let start = b.0.len();
    let mut input: Box<dyn Read + '_> = Box::new(std::io::empty());
    for table in &tables {
        input = Box::new(input.chain(table.data));
    }
    let params = brotli::enc::BrotliEncoderParams {
        quality: 6,
        lgwin: 20, // Bounded 1 MiB window; balance large-project time and size.
        mode: brotli::enc::backward_references::BrotliEncoderMode::BROTLI_MODE_FONT,
        ..Default::default()
    };
    brotli::BrotliCompress(&mut input, &mut b.0, &params).map_err(|e| e.to_string())?;
    b.put(20, size(b.0.len() - start)?);
    // Chromium's reference decoder rounds the end of the compressed block to four bytes.
    while b.0.len() % 4 != 0 {
        b.0.push(0);
    }
    b.put(8, size(b.0.len())?);
    Ok(b.0)
}
