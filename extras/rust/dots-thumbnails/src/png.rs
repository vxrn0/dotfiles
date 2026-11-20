use std::io::Cursor;

const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut checked = Vec::with_capacity(kind.len() + data.len());
    checked.extend_from_slice(kind);
    checked.extend_from_slice(data);
    out.extend_from_slice(&crc32(&checked).to_be_bytes());
    out
}

fn text_chunk(keyword: &str, text: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(keyword.len() + text.len() + 1);
    data.extend_from_slice(keyword.as_bytes());
    data.push(0);
    data.extend_from_slice(text.as_bytes());
    chunk(b"tEXt", &data)
}

fn parse_chunks(png: &[u8]) -> Option<Vec<(&[u8; 4], &[u8])>> {
    if png.len() < 8 || &png[..8] != SIGNATURE {
        return None;
    }
    let mut chunks = Vec::new();
    let mut offset = 8;
    while offset + 12 <= png.len() {
        let length = u32::from_be_bytes(png[offset..offset + 4].try_into().ok()?) as usize;
        if offset + 12 + length > png.len() {
            return None;
        }
        let kind: &[u8; 4] = png[offset + 4..offset + 8].try_into().ok()?;
        let data = &png[offset + 8..offset + 8 + length];
        chunks.push((kind, data));
        offset += 12 + length;
        if kind == b"IEND" {
            break;
        }
    }
    Some(chunks)
}

fn text_value<'a>(chunks: &'a [(&[u8; 4], &[u8])], keyword: &str) -> Option<&'a str> {
    for (kind, data) in chunks {
        if *kind != b"tEXt" {
            continue;
        }
        let split = data.iter().position(|byte| *byte == 0)?;
        if &data[..split] == keyword.as_bytes() {
            return std::str::from_utf8(&data[split + 1..]).ok();
        }
    }
    None
}

/// The checks `gnome_desktop_thumbnail_is_valid` performs, plus a full decode
/// because the C version loads the whole PNG before checking its options.
pub fn valid(png: &[u8], uri: &str, mtime: i64) -> bool {
    let Some(chunks) = parse_chunks(png) else {
        return false;
    };
    if text_value(&chunks, "Thumb::URI") != Some(uri) {
        return false;
    }
    let Some(stored) =
        text_value(&chunks, "Thumb::MTime").and_then(|text| text.parse::<i64>().ok())
    else {
        return false;
    };
    if stored != mtime {
        return false;
    }
    let decoder = png::Decoder::new(Cursor::new(png));
    let Ok(mut reader) = decoder.read_info() else {
        return false;
    };
    let mut buffer = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buffer).is_ok()
}

/// Rebuild a thumbnailer PNG with the Freedesktop text chunks, keeping the
/// original image data untouched. All other text chunks are dropped, matching
/// what gdk-pixbuf writes back.
pub fn with_metadata(png: &[u8], uri: &str, mtime: i64) -> Option<Vec<u8>> {
    let chunks = parse_chunks(png)?;
    let image_width = text_value(&chunks, "Thumb::Image::Width");
    let image_height = text_value(&chunks, "Thumb::Image::Height");

    let mut out = Vec::with_capacity(png.len() + 256);
    out.extend_from_slice(SIGNATURE);
    let mut inserted = false;
    for (kind, data) in &chunks {
        if *kind == b"tEXt" || *kind == b"iTXt" || *kind == b"zTXt" {
            continue;
        }
        out.extend_from_slice(&chunk(kind, data));
        if *kind == b"IHDR" && !inserted {
            if let Some(width) = image_width {
                out.extend_from_slice(&text_chunk("Thumb::Image::Width", width));
            }
            if let Some(height) = image_height {
                out.extend_from_slice(&text_chunk("Thumb::Image::Height", height));
            }
            out.extend_from_slice(&text_chunk("Software", "GNOME::ThumbnailFactory"));
            out.extend_from_slice(&text_chunk("Thumb::MTime", &mtime.to_string()));
            out.extend_from_slice(&text_chunk("Thumb::URI", uri));
            inserted = true;
        }
    }
    inserted.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_pixel_png() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[255, 0, 0, 255]).unwrap();
        }
        out
    }

    #[test]
    fn adds_the_required_chunks_and_keeps_pixels() {
        let source = one_pixel_png();
        let tagged = with_metadata(&source, "file:///tmp/a.png", 1234).unwrap();
        assert!(valid(&tagged, "file:///tmp/a.png", 1234));
        assert!(!valid(&tagged, "file:///tmp/a.png", 1235));
        assert!(!valid(&tagged, "file:///tmp/b.png", 1234));

        let decoder = png::Decoder::new(Cursor::new(&tagged));
        let mut reader = decoder.read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut pixels).unwrap();
        assert_eq!(&pixels[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn rejects_an_unrelated_file() {
        assert!(!valid(b"not a png", "file:///tmp/a.png", 1));
        assert!(with_metadata(b"not a png", "file:///tmp/a.png", 1).is_none());
    }
}
