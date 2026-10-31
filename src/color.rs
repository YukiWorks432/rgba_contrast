use crate::contrast::{Curve, LinearContrast, SigmoidCurves};
use after_effects::PixelF32;
use palette::convert::FromColorUnclamped;
use palette::{Hsl, Hsv, LinSrgb, Oklab, Oklch, Srgb};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Space {
    Rgb,
    Hsl,
    Hsv,
    Yuv,
    Oklab,
    Oklch,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Transfer {
    Srgb,
    Rec709Display,
    Rec709,
    Linear,
}

pub(crate) struct Adjustment {
    pub space: Space,
    pub transfer: Transfer,
    pub modern: bool,
    pub contrast: [f32; 4],
    pub pivot: [f32; 4],
    pub gamma: [f32; 4],
    pub hue_origin: f32,
    pub separate_alpha: bool,
    pub premultiplied: bool,
    curves: [Curve; 4],
    old_curves: SigmoidCurves,
}
impl Adjustment {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        space: Space,
        transfer: Transfer,
        modern: bool,
        contrast: [f32; 4],
        pivot: [f32; 4],
        gamma: [f32; 4],
        hue_origin: f32,
        separate_alpha: bool,
        premultiplied: bool,
    ) -> Self {
        let curves = std::array::from_fn(|i| Curve::new(contrast[i] / 5.0, pivot[i]));
        let old_curves = SigmoidCurves::new(
            PixelF32 {
                red: contrast[0] / 5.0,
                green: contrast[1] / 5.0,
                blue: contrast[2] / 5.0,
                alpha: contrast[3] / 5.0,
            },
            PixelF32 {
                red: pivot[0],
                green: pivot[1],
                blue: pivot[2],
                alpha: pivot[3],
            },
        );
        Self {
            space,
            transfer,
            modern,
            contrast,
            pivot,
            gamma,
            hue_origin,
            separate_alpha,
            premultiplied,
            curves,
            old_curves,
        }
    }
    fn contrast_axis(&self, x: f64, i: usize) -> f64 {
        if self.contrast[i] == 0.0 {
            return x;
        }
        if self.modern {
            self.curves[i].apply(x as f32) as f64
        } else {
            self.pivot[i] as f64
                + (x - self.pivot[i] as f64) * (1.0 + self.contrast[i] as f64 / 100.0)
        }
    }
    fn gamma_axis(&self, x: f64, i: usize, scale: f64) -> f64 {
        if self.gamma[i] == 1.0 || x == 0.0 {
            return x;
        }
        x.signum() * scale * (x.abs() / scale).powf(1.0 / self.gamma[i] as f64)
    }
    pub fn apply(&self, pixel: PixelF32) -> PixelF32 {
        let color_active = (0..3).any(|i| self.contrast[i] != 0.0 || self.gamma[i] != 1.0);
        let alpha_active = self.contrast[3] != 0.0 || self.gamma[3] != 1.0;
        if !color_active && !alpha_active {
            return pixel;
        }
        if self.space == Space::Rgb && !self.separate_alpha && self.gamma == [1.0; 4] {
            return if self.modern {
                self.old_curves.apply(pixel)
            } else {
                pixel.linear_contrast(
                    PixelF32 {
                        red: 1.0 + self.contrast[0] / 100.0,
                        green: 1.0 + self.contrast[1] / 100.0,
                        blue: 1.0 + self.contrast[2] / 100.0,
                        alpha: 1.0 + self.contrast[3] / 100.0,
                    },
                    PixelF32 {
                        red: self.pivot[0],
                        green: self.pivot[1],
                        blue: self.pivot[2],
                        alpha: self.pivot[3],
                    },
                )
            };
        }
        let a = pixel.alpha as f64;
        let mut ao = self.contrast_axis(a, 3);
        if self.gamma[3] != 1.0 {
            ao = self.gamma_axis(ao.clamp(0.0, 1.0), 3, 1.0);
        }
        let premul = self.separate_alpha && self.premultiplied;
        let raw = [pixel.red as f64, pixel.green as f64, pixel.blue as f64];
        if premul && a == 0.0 && ao == 0.0 {
            return PixelF32 {
                alpha: ao as f32,
                ..pixel
            };
        }
        let mut rgb = if premul {
            if a > 0.0 {
                raw.map(|v| v / a)
            } else {
                [0.0; 3]
            }
        } else {
            raw
        };
        if color_active {
            if self.space == Space::Hsl {
                rgb = rgb.map(|x| x.clamp(0.0, 1.0));
            }
            if self.space == Space::Hsv {
                rgb = rgb.map(|x| x.max(0.0));
            }
            let neutral = rgb[0] == rgb[1] && rgb[1] == rgb[2];
            let mut axes = self.forward(rgb);
            if neutral {
                match self.space {
                    Space::Oklab | Space::Yuv => {
                        axes[1] = 0.0;
                        axes[2] = 0.0;
                    }
                    Space::Oklch => {
                        axes[1] = 0.0;
                        axes[2] = 0.0;
                    }
                    _ => (),
                }
            }
            let hue = match self.space {
                Space::Hsl | Space::Hsv => Some(0),
                Space::Oklch => Some(2),
                _ => None,
            };
            for (i, value) in axes.iter_mut().enumerate() {
                if Some(i) == hue {
                    let h = *value
                        + (180.0 / std::f64::consts::PI)
                            * (self.contrast[i] as f64 / 100.0)
                            * ((*value - self.pivot[i] as f64 * 360.0).to_radians()).sin();
                    let u = (h - self.hue_origin as f64).rem_euclid(360.0) / 360.0;
                    *value = (self.hue_origin as f64 + 360.0 * u.powf(1.0 / self.gamma[i] as f64))
                        .rem_euclid(360.0);
                    continue;
                }
                let signed_scale = match (self.space, i) {
                    (Space::Yuv, 1 | 2) => Some(0.5),
                    (Space::Oklab, 1 | 2) => Some(0.4),
                    _ => None,
                };
                *value = if let Some(scale) = signed_scale {
                    (self.contrast_axis((*value / scale + 1.0) / 2.0, i) * 2.0 - 1.0) * scale
                } else {
                    self.contrast_axis(*value, i)
                };
                match (self.space, i) {
                    (Space::Hsl, 1 | 2) | (Space::Hsv, 1) => *value = value.clamp(0.0, 1.0),
                    (Space::Hsv, 2) | (Space::Oklch, 1) => *value = value.max(0.0),
                    _ => (),
                }
                let scale = signed_scale.unwrap_or(if self.space == Space::Oklch && i == 1 {
                    0.4
                } else {
                    1.0
                });
                *value = self.gamma_axis(*value, i, scale);
            }
            rgb = self.inverse(axes);
        }
        if premul {
            rgb = rgb.map(|v| v * ao);
        }
        PixelF32 {
            red: rgb[0] as f32,
            green: rgb[1] as f32,
            blue: rgb[2] as f32,
            alpha: ao as f32,
        }
    }
    fn forward(&self, mut rgb: [f64; 3]) -> [f64; 3] {
        if self.space == Space::Rgb {
            return rgb;
        }
        rgb = rgb.map(|x| decode(x, self.transfer));
        match self.space {
            Space::Hsl | Space::Hsv => {
                let c = rgb.map(|x| encode(x, Transfer::Srgb));
                let hsv: Hsv<palette::encoding::Srgb, f64> =
                    Hsv::from_color_unclamped(Srgb::new(c[0], c[1], c[2]));
                let h = hsv.hue.into_positive_degrees();
                if self.space == Space::Hsv {
                    [h, hsv.saturation, hsv.value]
                } else {
                    let max = c[0].max(c[1]).max(c[2]);
                    let min = c[0].min(c[1]).min(c[2]);
                    let d = max - min;
                    let s = if d == 0.0 {
                        0.0
                    } else if max + min <= 1.0 {
                        d / (max + min)
                    } else {
                        d / ((1.0 - max) + (1.0 - min))
                    };
                    [h, s, (max + min) / 2.0]
                }
            }
            Space::Yuv => {
                let c = rgb.map(|x| encode(x, Transfer::Rec709));
                let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                [y, (c[2] - y) / 1.8556, (c[0] - y) / 1.5748]
            }
            Space::Oklab | Space::Oklch => {
                let lab: Oklab<f64> =
                    Oklab::from_color_unclamped(LinSrgb::new(rgb[0], rgb[1], rgb[2]));
                if self.space == Space::Oklab {
                    [lab.l, lab.a, lab.b]
                } else {
                    let lch: Oklch<f64> = Oklch::from_color_unclamped(lab);
                    [lch.l, lch.chroma, lch.hue.into_positive_degrees()]
                }
            }
            Space::Rgb => unreachable!(),
        }
    }
    fn inverse(&self, v: [f64; 3]) -> [f64; 3] {
        if self.space == Space::Rgb {
            return v;
        }
        let linear = match self.space {
            Space::Hsl => {
                let c: Srgb<f64> = Srgb::from_color_unclamped(Hsl::new(v[0], v[1], v[2]));
                [c.red, c.green, c.blue].map(|x| decode(x, Transfer::Srgb))
            }
            Space::Hsv => {
                let c: Srgb<f64> = Srgb::from_color_unclamped(Hsv::new(v[0], v[1], v[2]));
                [c.red, c.green, c.blue].map(|x| decode(x, Transfer::Srgb))
            }
            Space::Yuv => {
                let r = v[0] + 1.5748 * v[2];
                let b = v[0] + 1.8556 * v[1];
                let g = (v[0] - 0.2126 * r - 0.0722 * b) / 0.7152;
                [r, g, b].map(|x| decode(x, Transfer::Rec709))
            }
            Space::Oklab => {
                let c: LinSrgb<f64> = LinSrgb::from_color_unclamped(Oklab::new(v[0], v[1], v[2]));
                [c.red, c.green, c.blue]
            }
            Space::Oklch => {
                let c: LinSrgb<f64> = LinSrgb::from_color_unclamped(Oklch::new(v[0], v[1], v[2]));
                [c.red, c.green, c.blue]
            }
            Space::Rgb => unreachable!(),
        };
        linear.map(|x| encode(x, self.transfer))
    }
}
fn decode(x: f64, t: Transfer) -> f64 {
    let v = x.abs();
    let y = match t {
        Transfer::Linear => v,
        Transfer::Srgb => {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        Transfer::Rec709Display => v.powf(2.4),
        Transfer::Rec709 => {
            if v < 0.081 {
                v / 4.5
            } else {
                ((v + 0.099) / 1.099).powf(1.0 / 0.45)
            }
        }
    };
    x.signum() * y
}
fn encode(x: f64, t: Transfer) -> f64 {
    let v = x.abs();
    let y = match t {
        Transfer::Linear => v,
        Transfer::Srgb => {
            if v <= 0.0031308 {
                12.92 * v
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            }
        }
        Transfer::Rec709Display => v.powf(1.0 / 2.4),
        Transfer::Rec709 => {
            if v < 0.018 {
                4.5 * v
            } else {
                1.099 * v.powf(0.45) - 0.099
            }
        }
    };
    x.signum() * y
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(space: Space) -> Adjustment {
        Adjustment::new(
            space,
            Transfer::Srgb,
            false,
            [0.0; 4],
            [0.5; 4],
            [1.0; 4],
            0.0,
            false,
            false,
        )
    }
    fn pixel(r: f32, g: f32, b: f32, a: f32) -> PixelF32 {
        PixelF32 {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }
    fn close(p: PixelF32, expected: [f32; 4]) {
        for (x, y) in [p.red, p.green, p.blue, p.alpha].into_iter().zip(expected) {
            assert!((x - y).abs() < 0.00002, "actual {x} expected {y}");
        }
    }
    #[test]
    fn neutral_controls_preserve_hdr_and_hidden_rgb() {
        for space in [
            Space::Rgb,
            Space::Hsl,
            Space::Hsv,
            Space::Yuv,
            Space::Oklab,
            Space::Oklch,
        ] {
            let out = config(space).apply(pixel(-0.2, 1.5, 0.25, 0.0));
            close(out, [-0.2, 1.5, 0.25, 0.0]);
        }
    }
    #[test]
    fn hsl_and_hsv_hdr_contract() {
        let mut hsv = config(Space::Hsv);
        hsv.gamma[2] = 2.0;
        close(
            hsv.apply(pixel(1.5, 0.5, 0.5, 1.0)),
            [1.2247449, 0.4082483, 0.4082483, 1.0],
        );
        let mut hsl = config(Space::Hsl);
        hsl.gamma[2] = 2.0;
        close(
            hsl.apply(pixel(1.5, 0.5, 0.5, 1.0)),
            [1.0, 0.7320508, 0.7320508, 1.0],
        );
        let white = hsl.apply(pixel(
            1.0,
            1.0 - f32::EPSILON / 2.0,
            1.0 - f32::EPSILON / 2.0,
            1.0,
        ));
        assert!(white.red.is_finite() && white.green.is_finite() && white.blue.is_finite());
    }
    #[test]
    fn hue_gamma_redistributes_hue() {
        let mut a = config(Space::Hsv);
        a.gamma[0] = 2.0;
        close(a.apply(pixel(0.5, 1.0, 0.0, 1.0)), [0.0, 1.0, 1.0, 1.0]);
    }
    #[test]
    fn unassociated_half_transparent_color_is_not_unmultiplied() {
        let mut a = config(Space::Hsv);
        a.separate_alpha = true;
        a.gamma[2] = 2.0;
        close(
            a.apply(pixel(0.8, 0.2, 0.1, 0.25)),
            [0.8944272, 0.2236068, 0.1118034, 0.25],
        );
    }
    #[test]
    fn opacity_change_preserves_unassociated_color() {
        let mut a = config(Space::Hsl);
        a.separate_alpha = true;
        a.premultiplied = true;
        a.gamma[3] = 2.0;
        close(
            a.apply(pixel(0.2, 0.05, 0.025, 0.25)),
            [0.4, 0.1, 0.05, 0.5],
        );
        close(
            a.apply(pixel(0.5, 0.25, 0.125, 0.25)),
            [1.0, 0.5, 0.25, 0.5],
        );
        close(a.apply(pixel(0.1, 0.2, 0.3, 0.0)), [0.1, 0.2, 0.3, 0.0]);
    }
    #[test]
    fn neutral_colors_remain_neutral_with_chroma_gamma() {
        for space in [Space::Yuv, Space::Oklab, Space::Oklch] {
            let mut a = config(space);
            a.gamma[1] = 10.0;
            a.gamma[2] = 10.0;
            let p = a.apply(pixel(0.3, 0.3, 0.3, 1.0));
            assert!(
                (p.red - p.green).abs() < 0.00002 && (p.green - p.blue).abs() < 0.00002,
                "{space:?}: {p:?}"
            );
        }
    }
    #[test]
    fn conversion_working_space_round_trips() {
        for space in [
            Space::Hsl,
            Space::Hsv,
            Space::Yuv,
            Space::Oklab,
            Space::Oklch,
        ] {
            for transfer in [
                Transfer::Srgb,
                Transfer::Rec709Display,
                Transfer::Rec709,
                Transfer::Linear,
            ] {
                let mut a = config(space);
                a.transfer = transfer;
                let input = [0.2, 0.4, 0.7];
                let output = a.inverse(a.forward(input));
                for (v, w) in output.into_iter().zip(input) {
                    assert!((v - w).abs() < 0.0001, "{space:?} {transfer:?}: {v} {w}");
                }
            }
        }
    }
}
