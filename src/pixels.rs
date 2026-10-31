use ae::{Pixel8, Pixel16, PixelF32};
use after_effects as ae;

pub(crate) trait ToPixel {
    fn to_pixel32(&self) -> PixelF32;
    fn to_pixel16(&self) -> Pixel16;
    fn to_pixel8(&self) -> Pixel8;
}
impl ToPixel for Pixel8 {
    fn to_pixel32(&self) -> PixelF32 {
        PixelF32 {
            red: self.red as f32 / ae::MAX_CHANNEL8 as f32,
            green: self.green as f32 / ae::MAX_CHANNEL8 as f32,
            blue: self.blue as f32 / ae::MAX_CHANNEL8 as f32,
            alpha: self.alpha as f32 / ae::MAX_CHANNEL8 as f32,
        }
    }
    fn to_pixel16(&self) -> Pixel16 {
        Pixel16 {
            red: (self.red as f32 / ae::MAX_CHANNEL8 as f32 * ae::MAX_CHANNEL16 as f32).round()
                as u16,
            green: (self.green as f32 / ae::MAX_CHANNEL8 as f32 * ae::MAX_CHANNEL16 as f32).round()
                as u16,
            blue: (self.blue as f32 / ae::MAX_CHANNEL8 as f32 * ae::MAX_CHANNEL16 as f32).round()
                as u16,
            alpha: (self.alpha as f32 / ae::MAX_CHANNEL8 as f32 * ae::MAX_CHANNEL16 as f32).round()
                as u16,
        }
    }
    fn to_pixel8(&self) -> Pixel8 {
        *self
    }
}
impl ToPixel for Pixel16 {
    fn to_pixel32(&self) -> PixelF32 {
        PixelF32 {
            red: self.red as f32 / ae::MAX_CHANNEL16 as f32,
            green: self.green as f32 / ae::MAX_CHANNEL16 as f32,
            blue: self.blue as f32 / ae::MAX_CHANNEL16 as f32,
            alpha: self.alpha as f32 / ae::MAX_CHANNEL16 as f32,
        }
    }
    fn to_pixel16(&self) -> Pixel16 {
        *self
    }
    fn to_pixel8(&self) -> Pixel8 {
        Pixel8 {
            red: (self.red as f32 / ae::MAX_CHANNEL16 as f32 * ae::MAX_CHANNEL8 as f32).round()
                as u8,
            green: (self.green as f32 / ae::MAX_CHANNEL16 as f32 * ae::MAX_CHANNEL8 as f32).round()
                as u8,
            blue: (self.blue as f32 / ae::MAX_CHANNEL16 as f32 * ae::MAX_CHANNEL8 as f32).round()
                as u8,
            alpha: (self.alpha as f32 / ae::MAX_CHANNEL16 as f32 * ae::MAX_CHANNEL8 as f32).round()
                as u8,
        }
    }
}
impl ToPixel for PixelF32 {
    fn to_pixel32(&self) -> PixelF32 {
        *self
    }
    fn to_pixel16(&self) -> Pixel16 {
        Pixel16 {
            red: (self.red.clamp(0.0, 1.0) * ae::MAX_CHANNEL16 as f32).round() as u16,
            green: (self.green.clamp(0.0, 1.0) * ae::MAX_CHANNEL16 as f32).round() as u16,
            blue: (self.blue.clamp(0.0, 1.0) * ae::MAX_CHANNEL16 as f32).round() as u16,
            alpha: (self.alpha.clamp(0.0, 1.0) * ae::MAX_CHANNEL16 as f32).round() as u16,
        }
    }
    fn to_pixel8(&self) -> Pixel8 {
        Pixel8 {
            red: (self.red.clamp(0.0, 1.0) * ae::MAX_CHANNEL8 as f32).round() as u8,
            green: (self.green.clamp(0.0, 1.0) * ae::MAX_CHANNEL8 as f32).round() as u8,
            blue: (self.blue.clamp(0.0, 1.0) * ae::MAX_CHANNEL8 as f32).round() as u8,
            alpha: (self.alpha.clamp(0.0, 1.0) * ae::MAX_CHANNEL8 as f32).round() as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contrast::LinearContrast;
    #[test]
    fn neutral_legacy_preserves_every_integer_value() {
        let gain = PixelF32 {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            alpha: 1.0,
        };
        let pivot = PixelF32 {
            red: 0.5,
            green: 0.25,
            blue: 0.75,
            alpha: 0.5,
        };
        for v in 0..=255u8 {
            let p = Pixel8 {
                red: v,
                green: v,
                blue: v,
                alpha: v,
            };
            let out = p.to_pixel32().linear_contrast(gain, pivot).to_pixel8();
            assert_eq!((out.red, out.green, out.blue, out.alpha), (v, v, v, v));
        }
        for v in 0..=32768u16 {
            let p = Pixel16 {
                red: v,
                green: v,
                blue: v,
                alpha: v,
            };
            let out = p.to_pixel32().linear_contrast(gain, pivot).to_pixel16();
            assert_eq!((out.red, out.green, out.blue, out.alpha), (v, v, v, v));
        }
    }
}
