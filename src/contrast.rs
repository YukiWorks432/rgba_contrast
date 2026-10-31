use after_effects::PixelF32;

pub(crate) trait LinearContrast {
    fn linear_contrast(&self, gain: PixelF32, pivot: PixelF32) -> Self;
}

impl LinearContrast for PixelF32 {
    fn linear_contrast(&self, gain: PixelF32, pivot: PixelF32) -> Self {
        fn channel(x: f32, gain: f32, pivot: f32) -> f32 {
            if gain == 1.0 {
                x
            } else {
                (x - pivot) * gain + pivot
            }
        }
        Self {
            red: channel(self.red, gain.red, pivot.red),
            green: channel(self.green, gain.green, pivot.green),
            blue: channel(self.blue, gain.blue, pivot.blue),
            alpha: channel(self.alpha, gain.alpha, pivot.alpha),
        }
    }
}

// 端点は画素ごとに変わらないため、描画前に倍精度で求める。
#[derive(Clone, Copy)]
pub(crate) struct Curve {
    gain: f32,
    k: f64,
    pivot: f64,
    lo: f64,
    hi: f64,
}

impl Curve {
    pub(crate) fn new(gain: f32, pivot: f32) -> Self {
        let k = (gain as f64).abs();
        let pivot = pivot as f64;
        let logistic = |v: f64| 1.0 / (1.0 + (k * (pivot - v)).exp());
        Self {
            gain,
            k,
            pivot,
            lo: logistic(0.0),
            hi: logistic(1.0),
        }
    }

    pub(crate) fn apply(self, x: f32) -> f32 {
        if !self.gain.is_finite() || self.gain.abs() < 1e-7 {
            return x;
        }
        let x = x as f64;
        if self.gain > 0.0 {
            let s = 1.0 / (1.0 + (self.k * (self.pivot - x)).exp());
            ((s - self.lo) / (self.hi - self.lo)) as f32
        } else if x == 0.0 || x == 1.0 {
            x as f32
        } else {
            // 対数の定義域外になるHDR値は元の実装と同じ範囲制限を行う。
            let s = (self.lo + x * (self.hi - self.lo)).clamp(1e-12, 1.0 - 1e-12);
            (self.pivot + (s / (1.0 - s)).ln() / self.k) as f32
        }
    }
}

pub(crate) struct SigmoidCurves {
    red: Curve,
    green: Curve,
    blue: Curve,
    alpha: Curve,
}
impl SigmoidCurves {
    pub(crate) fn new(gain: PixelF32, pivot: PixelF32) -> Self {
        Self {
            red: Curve::new(gain.red, pivot.red),
            green: Curve::new(gain.green, pivot.green),
            blue: Curve::new(gain.blue, pivot.blue),
            alpha: Curve::new(gain.alpha, pivot.alpha),
        }
    }
    pub(crate) fn apply(&self, pixel: PixelF32) -> PixelF32 {
        PixelF32 {
            red: self.red.apply(pixel.red),
            green: self.green.apply(pixel.green),
            blue: self.blue.apply(pixel.blue),
            alpha: self.alpha.apply(pixel.alpha),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Curve;
    fn sigmoid(x: f32, gain: f32, pivot: f32) -> f32 {
        Curve::new(gain, pivot).apply(x)
    }

    #[test]
    fn finite_for_extreme_pivots_and_near_zero() {
        for p in [0.0, 0.1, 0.5, 0.9, 1.0] {
            for g in [-20.0, -4.0, -1e-8, 0.0, 1e-8, 4.0, 20.0] {
                let mut prev = -1.0;
                for i in 0..=100 {
                    let x = i as f32 / 100.0;
                    let y = sigmoid(x, g, p);
                    assert!(
                        y.is_finite() && (0.0..=1.0).contains(&y),
                        "{x} {g} {p}: {y}"
                    );
                    assert!(y >= prev, "{x} {g} {p}: {y} < {prev}");
                    prev = y;
                }
            }
        }
    }

    #[test]
    fn channel_controls_are_independent() {
        let pixel = after_effects::PixelF32 {
            red: 0.25,
            green: 0.4,
            blue: 1.5,
            alpha: 0.7,
        };
        let gain = after_effects::PixelF32 {
            red: 4.0,
            green: 0.0,
            blue: 0.0,
            alpha: 0.0,
        };
        let pivot = after_effects::PixelF32 {
            red: 0.25,
            green: 0.5,
            blue: 0.5,
            alpha: 0.5,
        };
        let out = super::SigmoidCurves::new(gain, pivot).apply(pixel);
        assert!((out.red - 0.3379864).abs() < 1e-6);
        assert_eq!(
            (out.green, out.blue, out.alpha),
            (pixel.green, pixel.blue, pixel.alpha)
        );
    }

    #[test]
    fn documented_curve_and_inverse() {
        // シグモイドの端点正規化から独立に算出した参照値。
        assert!((sigmoid(0.25, 4.0, 0.25) - 0.3379864).abs() < 1e-6);
        assert!((sigmoid(0.25, -4.0, 0.25) - 0.1895569).abs() < 1e-6);
        assert!((sigmoid(0.5, -20.0, 0.9) - 0.8880228).abs() < 1e-5);
        for p in [0.1, 0.5, 0.9] {
            for g in [0.5, 4.0, 10.0] {
                for x in [0.1, 0.25, 0.5, 0.75, 0.9] {
                    assert!((sigmoid(sigmoid(x, g, p), -g, p) - x).abs() < 1e-4);
                }
            }
        }
    }
}
