use after_effects as ae;

// A simple noise algorithm. This example shows how to handle different pixel formats and how to iterate them using rayon

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum Params {
  Red,
  Green,
  Blue,
  Alpha,
  Mode,
}
#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
#[repr(i32)]
enum Mode {
  Modern = 1,
  Legacy = 2,
}

impl core::convert::TryFrom<i32> for Mode {
  type Error = ();
  fn try_from(value: i32) -> Result<Self, Self::Error> {
    match value {
      1 => Ok(Mode::Modern),
      2 => Ok(Mode::Legacy),
      _ => Err(()),
    }
  }
}

#[derive(Default)]
struct Plugin {}

ae::define_effect!(Plugin, (), Params);

const EPS: f32 = 1e-12;

trait Linear {
  fn linear(&self, gain: f32, pivot: f32) -> Self;
}

impl Linear for f32 {
  fn linear(&self, gain: f32, pivot: f32) -> Self {
    (self - pivot) * gain + pivot
  }
}
trait LinearContrast {
  fn linear_contrast(&self, gain: PixelF32, pivot: f32) -> Self;
}
impl LinearContrast for PixelF32 {
  fn linear_contrast(&self, gain: PixelF32, pivot: f32) -> Self {
    Self {
      red: self.red.linear(gain.red, pivot),
      green: self.green.linear(gain.green, pivot),
      blue: self.blue.linear(gain.blue, pivot),
      alpha: self.alpha.linear(gain.alpha, pivot),
    }
  }
}
trait NormalSigmoid {
  fn normal_sigmoid(&self, gain: f32, pivot: f32, min: f32, max: f32) -> Self;
}
impl NormalSigmoid for f32 {
  fn normal_sigmoid(&self, gain: f32, pivot: f32, min: f32, max: f32) -> Self {
    if !gain.is_finite() {
      return *self;
    }
    if !min.is_finite() && !max.is_finite() {
      return 1.0 / (1.0 + (gain * (pivot - *self)).exp());
    }
    ((1.0 / (1.0 + (gain * (pivot - *self)).exp())) - min) / (max - min)
  }
}
trait InverseSigmoid {
  fn inverse_sigmoid(&self, gain: f32, pivot: f32, min: f32, max: f32) -> Self;
}
impl InverseSigmoid for f32 {
  fn inverse_sigmoid(&self, gain: f32, pivot: f32, min: f32, max: f32) -> Self {
    if !gain.is_finite() {
      return *self;
    }

    let s0 = 1.0 / (1.0 + (gain * pivot).exp());
    let s1 = 1.0 / (1.0 + (gain * (pivot - 1.0)).exp());
    let s = s0 + *self * (s1 - s0);

    // numerical safety
    let s_clamped = s.clamp(EPS, 1.0 - EPS);

    if !min.is_finite() && !max.is_finite() {
      return pivot - (1.0 / gain) * ((1.0 - s_clamped) / s_clamped).ln();
    }

    (pivot - (1.0 / gain) * ((1.0 - s_clamped) / s_clamped).ln() - min)
      / (max - min)
  }
}

trait Sigmoid {
  fn sigmoid(&self, gain: f32, pivot: f32, min: f32, max: f32) -> Self;
}
impl Sigmoid for f32 {
  fn sigmoid(&self, gain: f32, pivot: f32, min: f32, max: f32) -> Self {
    if gain > EPS {
      self.normal_sigmoid(gain, pivot, min, max)
    } else if gain < -EPS {
      self.inverse_sigmoid(gain, pivot, min, max)
    } else {
      *self
    }
  }
}

trait SigmoidContrast {
  fn sigmoid_contrast(
    &self,
    gain: PixelF32,
    pivot: f32,
    min: PixelF32,
    max: PixelF32,
  ) -> Self;
}
impl SigmoidContrast for PixelF32 {
  fn sigmoid_contrast(
    &self,
    gain: PixelF32,
    pivot: f32,
    min: PixelF32,
    max: PixelF32,
  ) -> Self {
    Self {
      red: self.red.sigmoid(gain.red, pivot, min.red, max.red),
      green: self.green.sigmoid(gain.green, pivot, min.green, max.green),
      blue: self.blue.sigmoid(gain.blue, pivot, min.blue, max.blue),
      alpha: self.alpha.sigmoid(gain.alpha, pivot, min.alpha, max.alpha),
    }
  }
}

trait ToPixel {
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
      red: (self.red as f32 / ae::MAX_CHANNEL8 as f32
        * ae::MAX_CHANNEL16 as f32) as u16,
      green: (self.green as f32 / ae::MAX_CHANNEL8 as f32
        * ae::MAX_CHANNEL16 as f32) as u16,
      blue: (self.blue as f32 / ae::MAX_CHANNEL8 as f32
        * ae::MAX_CHANNEL16 as f32) as u16,
      alpha: (self.alpha as f32 / ae::MAX_CHANNEL8 as f32
        * ae::MAX_CHANNEL16 as f32) as u16,
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
      red: (self.red as f32 / ae::MAX_CHANNEL16 as f32
        * ae::MAX_CHANNEL8 as f32) as u8,
      green: (self.green as f32 / ae::MAX_CHANNEL16 as f32
        * ae::MAX_CHANNEL8 as f32) as u8,
      blue: (self.blue as f32 / ae::MAX_CHANNEL16 as f32
        * ae::MAX_CHANNEL8 as f32) as u8,
      alpha: (self.alpha as f32 / ae::MAX_CHANNEL16 as f32
        * ae::MAX_CHANNEL8 as f32) as u8,
    }
  }
}
impl ToPixel for PixelF32 {
  fn to_pixel32(&self) -> PixelF32 {
    *self
  }
  fn to_pixel16(&self) -> Pixel16 {
    Pixel16 {
      red: (self.red.clamp(0.0, 1.0) * ae::MAX_CHANNEL16 as f32) as u16,
      green: (self.green.clamp(0.0, 1.0) * ae::MAX_CHANNEL16 as f32) as u16,
      blue: (self.blue.clamp(0.0, 1.0) * ae::MAX_CHANNEL16 as f32) as u16,
      alpha: (self.alpha.clamp(0.0, 1.0) * ae::MAX_CHANNEL16 as f32) as u16,
    }
  }
  fn to_pixel8(&self) -> Pixel8 {
    Pixel8 {
      red: (self.red.clamp(0.0, 1.0) * ae::MAX_CHANNEL8 as f32) as u8,
      green: (self.green.clamp(0.0, 1.0) * ae::MAX_CHANNEL8 as f32) as u8,
      blue: (self.blue.clamp(0.0, 1.0) * ae::MAX_CHANNEL8 as f32) as u8,
      alpha: (self.alpha.clamp(0.0, 1.0) * ae::MAX_CHANNEL8 as f32) as u8,
    }
  }
}

impl AdobePluginGlobal for Plugin {
  fn can_load(_host_name: &str, _host_version: &str) -> bool {
    true
  }

  fn params_setup(
    &self,
    params: &mut ae::Parameters<Params>,
    _in_data: InData,
    _: OutData,
  ) -> Result<(), Error> {
    let my_slider = |f: &mut ae::FloatSliderDef| {
      f.set_slider_min(-100.0);
      f.set_slider_max(100.0);
      f.set_valid_min(-100.0);
      f.set_valid_max(100.0);
      f.set_default(0.0);
      f.set_precision(2);
    };
    params.add(Params::Red, "Red", ae::FloatSliderDef::setup(my_slider))?;
    params.add(Params::Green, "Green", ae::FloatSliderDef::setup(my_slider))?;
    params.add(Params::Blue, "Blue", ae::FloatSliderDef::setup(my_slider))?;
    params.add(Params::Alpha, "Alpha", ae::FloatSliderDef::setup(my_slider))?;

    params.add(
      Params::Mode,
      "Mode",
      ae::PopupDef::setup(|f| {
        f.set_options(&["Modern", "Legacy"]);
        f.set_default(2);
        f.set_value(f.default());
      }),
    )?;

    Ok(())
  }

  fn handle_command(
    &mut self,
    cmd: ae::Command,
    in_data: InData,
    mut out_data: OutData,
    params: &mut ae::Parameters<Params>,
  ) -> Result<(), ae::Error> {
    let get_params = || {
      let mode_val = params.get(Params::Mode)?.as_popup()?.value();
      let mode =
        Mode::try_from(mode_val).map_err(|_| Error::BadCallbackParameter)?;
      let (coef, coef2) = match mode {
        Mode::Modern => (5.0, 0.0),
        Mode::Legacy => (100.0, 1.0),
      };
      let gain = PixelF32 {
        red: params.get(Params::Red)?.as_float_slider()?.value() as f32 / coef
          + coef2,
        green: params.get(Params::Green)?.as_float_slider()?.value() as f32
          / coef
          + coef2,
        blue: params.get(Params::Blue)?.as_float_slider()?.value() as f32
          / coef
          + coef2,
        alpha: params.get(Params::Alpha)?.as_float_slider()?.value() as f32
          / coef
          + coef2,
      };

      let nan = PixelF32 {
        red: f32::NAN,
        green: f32::NAN,
        blue: f32::NAN,
        alpha: f32::NAN,
      };

      let min_pixel = PixelF32 {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
      };
      let max_pixel = PixelF32 {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
        alpha: 1.0,
      };
      let min = min_pixel.sigmoid_contrast(gain, 0.5, nan, nan);
      let max = max_pixel.sigmoid_contrast(gain, 0.5, nan, nan);

      Ok::<_, Error>((mode, gain, min, max))
    };
    match cmd {
      ae::Command::About => {
        out_data.set_return_msg(
"ColorContrast\rCopyright 2025 HanaYuki.\rThis allows you to adjust the contrast for each color.",
);
      }
      ae::Command::GlobalSetup => {
        // For Premiere - declare supported pixel formats
        if in_data.is_premiere() {
          let suite = ae::pf::suites::PixelFormat::new()?;

          // Add the pixel formats we support in order of preference.
          suite.clear_supported_pixel_formats(in_data.effect_ref())?;
          let formats = [
            ae::pr::PixelFormat::Bgra4444_32f,
            ae::pr::PixelFormat::Bgra4444_16u,
            ae::pr::PixelFormat::Bgra4444_8u,
          ];
          for x in formats {
            suite.add_supported_pixel_format(in_data.effect_ref(), x)?;
          }
        }
      }
      ae::Command::Render {
        in_layer,
        mut out_layer,
      } => {
        let progress_final = out_layer.height() as _;

        let (mode, gain, min, max) = get_params()?;

        in_layer.iterate_with(
          &mut out_layer,
          0,
          progress_final,
          None,
          |_x: i32,
           _y: i32,
           pixel: ae::GenericPixel,
           out_pixel: ae::GenericPixelMut|
           -> Result<(), Error> {
            match (pixel, out_pixel) {
              (
                ae::GenericPixel::Pixel8(pixel),
                ae::GenericPixelMut::Pixel8(out_pixel),
              ) => match mode {
                Mode::Modern => {
                  *out_pixel = pixel
                    .to_pixel32()
                    .sigmoid_contrast(gain, 0.5, min, max)
                    .to_pixel8()
                }
                Mode::Legacy => {
                  *out_pixel =
                    pixel.to_pixel32().linear_contrast(gain, 0.5).to_pixel8()
                }
              },
              (
                ae::GenericPixel::Pixel16(pixel),
                ae::GenericPixelMut::Pixel16(out_pixel),
              ) => match mode {
                Mode::Modern => {
                  *out_pixel = pixel
                    .to_pixel32()
                    .sigmoid_contrast(gain, 0.5, min, max)
                    .to_pixel16()
                }
                Mode::Legacy => {
                  *out_pixel =
                    pixel.to_pixel32().linear_contrast(gain, 0.5).to_pixel16()
                }
              },
              _ => return Err(Error::BadCallbackParameter),
            }
            Ok(())
          },
        )?;
      }
      ae::Command::SmartPreRender { mut extra } => {
        let req = extra.output_request();

        if let Ok(in_result) = extra.callbacks().checkout_layer(
          0,
          0,
          &req,
          in_data.current_time(),
          in_data.time_step(),
          in_data.time_scale(),
        ) {
          let _ = extra.union_result_rect(in_result.result_rect.into());
          let _ = extra.union_max_result_rect(in_result.max_result_rect.into());
        }
      }
      ae::Command::SmartRender { extra } => {
        let cb = extra.callbacks();
        let input_world = cb.checkout_layer_pixels(0)?;
        let mut output_world = cb.checkout_output()?;

        let progress_final = output_world.height() as _;

        let (mode, gain, min, max) = get_params()?;

        input_world.iterate_with(
          &mut output_world,
          0,
          progress_final,
          None,
          |_x: i32,
           _y: i32,
           pixel: ae::GenericPixel,
           out_pixel: ae::GenericPixelMut|
           -> Result<(), Error> {
            match (pixel, out_pixel) {
              (
                ae::GenericPixel::Pixel8(pixel),
                ae::GenericPixelMut::Pixel8(out_pixel),
              ) => match mode {
                Mode::Modern => {
                  *out_pixel = pixel
                    .to_pixel32()
                    .sigmoid_contrast(gain, 0.5, min, max)
                    .to_pixel8();
                }
                Mode::Legacy => {
                  *out_pixel =
                    pixel.to_pixel32().linear_contrast(gain, 0.5).to_pixel8()
                }
              },
              (
                ae::GenericPixel::Pixel16(pixel),
                ae::GenericPixelMut::Pixel16(out_pixel),
              ) => match mode {
                Mode::Modern => {
                  *out_pixel = pixel
                    .to_pixel32()
                    .sigmoid_contrast(gain, 0.5, min, max)
                    .to_pixel16();
                }
                Mode::Legacy => {
                  *out_pixel =
                    pixel.to_pixel32().linear_contrast(gain, 0.5).to_pixel16()
                }
              },
              (
                ae::GenericPixel::PixelF32(pixel),
                ae::GenericPixelMut::PixelF32(out_pixel),
              ) => match mode {
                Mode::Modern => {
                  *out_pixel = pixel.sigmoid_contrast(gain, 0.5, min, max);
                }
                Mode::Legacy => *out_pixel = pixel.linear_contrast(gain, 0.5),
              },
              _ => return Err(Error::BadCallbackParameter),
            }
            Ok(())
          },
        )?;

        cb.checkin_layer_pixels(0)?;
      }
      _ => {}
    }
    Ok(())
  }
}
