use crate::color::Adjustment;
use ae::{Error, PixelF32};
use after_effects as ae;

#[derive(Clone, Copy)]
enum Depth {
    U8,
    U16,
    F32,
}
fn convert(source: &[u8], output: &mut [u8], depth: Depth, adjustment: &Adjustment) {
    let read = |i: usize| -> f32 {
        match depth {
            Depth::U8 => source[i] as f32 / 255.0,
            Depth::U16 => u16::from_ne_bytes([source[i * 2], source[i * 2 + 1]]) as f32 / 32768.0,
            Depth::F32 => f32::from_ne_bytes(source[i * 4..i * 4 + 4].try_into().unwrap()),
        }
    };
    let p = adjustment.apply(PixelF32 {
        blue: read(0),
        green: read(1),
        red: read(2),
        alpha: read(3),
    });
    for (i, v) in [p.blue, p.green, p.red, p.alpha].into_iter().enumerate() {
        match depth {
            Depth::U8 => output[i] = (v.clamp(0.0, 1.0) * 255.0).round() as u8,
            Depth::U16 => output[i * 2..i * 2 + 2]
                .copy_from_slice(&((v.clamp(0.0, 1.0) * 32768.0).round() as u16).to_ne_bytes()),
            Depth::F32 => output[i * 4..i * 4 + 4].copy_from_slice(&v.to_ne_bytes()),
        }
    }
}
pub(crate) fn render(
    input: Option<&ae::Layer>,
    output: &mut ae::Layer,
    adjustment: &Adjustment,
) -> Result<(), Error> {
    let suite = ae::pf::suites::PixelFormat::new()?;
    let format = suite.pixel_format(&*output)?;
    if let Some(input) = input
        && suite.pixel_format(input)? != format
    {
        return Err(Error::BadCallbackParameter);
    }
    let (depth, bytes) = match format {
        ae::pr::PixelFormat::Bgra4444_8u => (Depth::U8, 4),
        ae::pr::PixelFormat::Bgra4444_16u => (Depth::U16, 8),
        ae::pr::PixelFormat::Bgra4444_32f => (Depth::F32, 16),
        _ => return Err(Error::BadCallbackParameter),
    };
    let width = output.width();
    let height = output.height();
    if width == 0 || height == 0 {
        return Ok(());
    }
    let output_origin = output.origin();
    let output_stride = output.buffer_stride();
    let output_row = output.row_bytes();
    let source = input.map(|i| {
        (
            i.buffer(),
            i.width(),
            i.height(),
            i.buffer_stride(),
            i.row_bytes(),
            i.origin(),
        )
    });
    let dst = output.buffer_mut();
    let zero = [0u8; 16];
    for y in 0..height {
        let oy = if output_row < 0 { height - 1 - y } else { y };
        for x in 0..width {
            let o = oy * output_stride + x * bytes;
            let pixel = source
                .as_ref()
                .and_then(|(data, w, h, stride, row, origin)| {
                    let ix = x as i64 + output_origin.h as i64 - origin.h as i64;
                    let iy = y as i64 + output_origin.v as i64 - origin.v as i64;
                    if ix < 0 || iy < 0 || ix >= *w as i64 || iy >= *h as i64 {
                        return None;
                    }
                    let iy = if *row < 0 {
                        *h - 1 - iy as usize
                    } else {
                        iy as usize
                    };
                    let start = iy * *stride + ix as usize * bytes;
                    Some(&data[start..start + bytes])
                })
                .unwrap_or(&zero[..bytes]);
            convert(pixel, &mut dst[o..o + bytes], depth, adjustment);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Space, Transfer};
    #[test]
    fn bgra_16_and_float_preserve_channel_order_and_range() {
        let a = Adjustment::new(
            Space::Rgb,
            Transfer::Srgb,
            false,
            [0.0; 4],
            [0.5; 4],
            [2.0, 1.0, 1.0, 1.0],
            0.0,
            false,
            false,
        );
        let mut source = [0u8; 8];
        for (i, v) in [4096u16, 8192, 8192, 32768].into_iter().enumerate() {
            source[i * 2..i * 2 + 2].copy_from_slice(&v.to_ne_bytes());
        }
        let mut output = [0u8; 8];
        convert(&source, &mut output, Depth::U16, &a);
        let values: Vec<_> = output
            .chunks_exact(2)
            .map(|c| u16::from_ne_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(values, [4096, 8192, 16384, 32768]);
        let mut source = [0u8; 16];
        for (i, v) in [-0.25f32, 1.5, 0.25, 0.5].into_iter().enumerate() {
            source[i * 4..i * 4 + 4].copy_from_slice(&v.to_ne_bytes());
        }
        let mut output = [0u8; 16];
        convert(&source, &mut output, Depth::F32, &a);
        let values: Vec<_> = output
            .chunks_exact(4)
            .map(|c| f32::from_ne_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(values, [-0.25, 1.5, 0.5, 0.5]);
    }
    #[test]
    fn bgra_channels_are_mapped_to_their_correct_axes() {
        let a = Adjustment::new(
            Space::Rgb,
            Transfer::Srgb,
            false,
            [0.0; 4],
            [0.5; 4],
            [2.0, 1.0, 1.0, 1.0],
            0.0,
            false,
            false,
        );
        let mut out = [0; 4];
        convert(&[32, 64, 64, 255], &mut out, Depth::U8, &a);
        assert_eq!(out, [32, 64, 128, 255]);
    }
}
