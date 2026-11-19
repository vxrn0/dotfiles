//! Image decoding shared by the colour-from-image path and `scheme-for-image`.
//!
//! JPEG is decoded through the system libjpeg-turbo (`libturbojpeg`), because
//! its integer IDCT and chroma upsampling are byte-identical to the libjpeg
//! build inside Pillow. PNG is lossless and goes through the `png` crate.

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_ulong};

#[link(name = "turbojpeg")]
extern "C" {
    fn tjInitDecompress() -> *mut c_void;
    fn tjDecompressHeader3(
        handle: *mut c_void,
        jpeg_buf: *mut u8,
        jpeg_size: c_ulong,
        width: *mut c_int,
        height: *mut c_int,
        jpeg_subsamp: *mut c_int,
        jpeg_colorspace: *mut c_int,
    ) -> c_int;
    fn tjDecompress(
        handle: *mut c_void,
        jpeg_buf: *mut u8,
        jpeg_size: c_ulong,
        dst_buf: *mut u8,
        width: c_int,
        pitch: c_int,
        height: c_int,
        pixel_size: c_int,
        flags: c_int,
    ) -> c_int;
    fn tjDestroy(handle: *mut c_void) -> c_int;
    fn tjGetErrorStr() -> *mut c_char;
}

pub struct Decoded {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
    /// Present only for a PNG whose Pillow mode is RGBA.
    pub alpha: Option<Vec<u8>>,
}

fn jpeg_error() -> String {
    unsafe {
        let ptr = tjGetErrorStr();
        if ptr.is_null() {
            "libjpeg-turbo error".to_string()
        } else {
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}

fn decode_jpeg(data: &[u8]) -> Result<Decoded, String> {
    unsafe {
        let handle = tjInitDecompress();
        if handle.is_null() {
            return Err("cannot initialise libjpeg-turbo".to_string());
        }
        let mut width: c_int = 0;
        let mut height: c_int = 0;
        let mut subsamp: c_int = 0;
        let mut colorspace: c_int = 0;
        let mut buf = data.to_vec();
        if tjDecompressHeader3(
            handle,
            buf.as_mut_ptr(),
            buf.len() as c_ulong,
            &mut width,
            &mut height,
            &mut subsamp,
            &mut colorspace,
        ) != 0
        {
            let error = jpeg_error();
            tjDestroy(handle);
            return Err(error);
        }
        // TJCS_CMYK = 3, TJCS_YCCK = 4.  Pillow keeps those in CMYK mode and
        // feeds the first three channels to the quantiser; libturbojpeg's
        // RGB conversion is not the same thing, so refuse instead of guessing.
        if colorspace == 3 || colorspace == 4 {
            tjDestroy(handle);
            return Err("CMYK JPEG is not supported".to_string());
        }
        if width <= 0 || height <= 0 {
            tjDestroy(handle);
            return Err("invalid JPEG dimensions".to_string());
        }
        let size = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(3))
            .ok_or_else(|| "JPEG is too large".to_string())?;
        let mut rgb = vec![0u8; size];
        if tjDecompress(
            handle,
            buf.as_mut_ptr(),
            buf.len() as c_ulong,
            rgb.as_mut_ptr(),
            width,
            width * 3,
            height,
            3,
            0,
        ) != 0
        {
            let error = jpeg_error();
            tjDestroy(handle);
            return Err(error);
        }
        tjDestroy(handle);
        Ok(Decoded {
            width: width as usize,
            height: height as usize,
            rgb,
            alpha: None,
        })
    }
}

fn decode_png(data: &[u8]) -> Result<Decoded, String> {
    // PNG IHDR: 8-byte signature, 4-byte length, "IHDR", width 4, height 4,
    // bit depth 1, colour type 1.
    if data.len() < 26 {
        return Err("truncated PNG".to_string());
    }
    let bit_depth = data[24];
    let color_type = data[25];
    if bit_depth == 16 {
        return Err("16-bit PNG is not supported".to_string());
    }
    if color_type == 4 {
        return Err("greyscale PNG with alpha is not supported".to_string());
    }

    let image = image::load_from_memory_with_format(data, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    let rgba = color_type == 6;
    let (width, height) = (image.width() as usize, image.height() as usize);
    let mut rgb = Vec::with_capacity(width * height * 3);
    let mut alpha = if rgba {
        Some(Vec::with_capacity(width * height))
    } else {
        None
    };
    if let Some(alpha) = alpha.as_mut() {
        for pixel in image.to_rgba8().pixels() {
            rgb.extend_from_slice(&pixel.0[..3]);
            alpha.push(pixel.0[3]);
        }
    } else {
        for pixel in image.to_rgb8().pixels() {
            rgb.extend_from_slice(&pixel.0);
        }
    }
    Ok(Decoded {
        width,
        height,
        rgb,
        alpha,
    })
}

fn decode_gif(data: &[u8], second_frame: bool) -> Result<Decoded, String> {
    use image::AnimationDecoder;

    let decoder = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(data))
        .map_err(|error| error.to_string())?;
    let mut frames = decoder.into_frames();
    let mut frame = frames
        .next()
        .ok_or_else(|| "GIF has no frames".to_string())?
        .map_err(|error| error.to_string())?;
    if second_frame {
        frame = frames
            .next()
            .ok_or_else(|| "no more images in GIF file".to_string())?
            .map_err(|error| error.to_string())?;
    }
    let buffer = frame.into_buffer();
    let (width, height) = buffer.dimensions();
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for pixel in buffer.pixels() {
        rgb.extend_from_slice(&pixel.0[..3]);
    }
    Ok(Decoded {
        width: width as usize,
        height: height as usize,
        rgb,
        alpha: None,
    })
}

fn decode_with(path: &str, gif_second_frame: bool) -> Result<Decoded, String> {
    let data = std::fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        decode_jpeg(&data)
    } else if data.len() >= 8 && data[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        decode_png(&data)
    } else if data.len() >= 6 && (&data[..6] == b"GIF87a" || &data[..6] == b"GIF89a") {
        decode_gif(&data, gif_second_frame)
    } else {
        Err(format!("{path}: unsupported image format"))
    }
}

/// Pillow's `Image.open` semantics, including `seek(1)` for a GIF.
pub fn decode(path: &str) -> Result<Decoded, String> {
    decode_with(path, true)
}

/// OpenCV's `imread` semantics: the first GIF frame.
pub fn decode_first(path: &str) -> Result<Decoded, String> {
    decode_with(path, false)
}

fn muldiv255(a: u8, b: u8) -> u8 {
    let tmp = a as u32 * b as u32 + 128;
    (((tmp >> 8) + tmp) >> 8) as u8
}

fn clip8(value: u32) -> u8 {
    if value < 256 {
        value as u8
    } else {
        255
    }
}

/// Resize an image the way `PIL.Image.resize(..., BICUBIC)` would.
///
/// RGBA images take Pillow's premultiplied path: convert to `RGBa`, resize all
/// four channels, then un-premultiply.  Pixel values are returned straight.
pub fn resize(
    rgb: &[u8],
    alpha: Option<&[u8]>,
    width: usize,
    height: usize,
    out_width: usize,
    out_height: usize,
) -> Vec<u8> {
    if let Some(alpha) = alpha {
        let mut rgba = Vec::with_capacity(width * height * 4);
        for i in 0..width * height {
            let a = alpha[i];
            rgba.push(muldiv255(rgb[i * 3], a));
            rgba.push(muldiv255(rgb[i * 3 + 1], a));
            rgba.push(muldiv255(rgb[i * 3 + 2], a));
            rgba.push(a);
        }
        let resized = crate::resize::bicubic(&rgba, width, height, out_width, out_height, 4);
        let mut out = Vec::with_capacity(out_width * out_height * 3);
        for pixel in resized.chunks_exact(4) {
            let a = pixel[3] as u32;
            if a == 0 || a == 255 {
                out.extend_from_slice(&pixel[..3]);
            } else {
                out.push(clip8(255 * pixel[0] as u32 / a));
                out.push(clip8(255 * pixel[1] as u32 / a));
                out.push(clip8(255 * pixel[2] as u32 / a));
            }
        }
        out
    } else {
        crate::resize::bicubic(rgb, width, height, out_width, out_height, 3)
    }
}
