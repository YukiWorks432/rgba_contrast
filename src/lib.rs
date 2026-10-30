#![allow(
  clippy::drop_non_drop,
  clippy::question_mark,
  reason = "after-effects の外部マクロ展開で発生するため"
)]

use after_effects as ae;
mod contrast;
mod pixels;
use contrast::{LinearContrast, SigmoidCurves};
use pixels::ToPixel;

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum Params {
  Red,
  Green,
  Blue,
  Alpha,
  Mode,
  RPivot,
  GPivot,
  BPivot,
  APivot,
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

impl AdobePluginGlobal for Plugin {
  fn params_setup(
    &self,
    params: &mut ae::Parameters<Params>,
    _in_data: InData,
    _: OutData,
  ) -> Result<(), Error> {
    fn gain_slider(f: &mut ae::FloatSliderDef) {
      f.set_slider_min(-100.0);
      f.set_slider_max(100.0);
      f.set_valid_min(-100.0);
      f.set_valid_max(100.0);
      f.set_default(0.0);
      f.set_precision(2);
    }
    params.add(Params::Red, "Red", ae::FloatSliderDef::setup(gain_slider))?;
    params.add(
      Params::Green,
      "Green",
      ae::FloatSliderDef::setup(gain_slider),
    )?;
    params.add(Params::Blue, "Blue", ae::FloatSliderDef::setup(gain_slider))?;
    params.add(
      Params::Alpha,
      "Alpha",
      ae::FloatSliderDef::setup(gain_slider),
    )?;

    fn pivot_slider(f: &mut ae::FloatSliderDef) {
      f.set_slider_min(0.0);
      f.set_slider_max(100.0);
      f.set_valid_min(0.0);
      f.set_valid_max(100.0);
      f.set_default(50.0);
      f.set_precision(2);
      f.set_display_flags(ae::ValueDisplayFlag::PERCENT);
    }

    params.add(
      Params::RPivot,
      "Red Pivot",
      ae::FloatSliderDef::setup(pivot_slider),
    )?;
    params.add(
      Params::GPivot,
      "Green Pivot",
      ae::FloatSliderDef::setup(pivot_slider),
    )?;
    params.add(
      Params::BPivot,
      "Blue Pivot",
      ae::FloatSliderDef::setup(pivot_slider),
    )?;
    params.add(
      Params::APivot,
      "Alpha Pivot",
      ae::FloatSliderDef::setup(pivot_slider),
    )?;

    params.add(
      Params::Mode,
      "Mode",
      ae::PopupDef::setup(|f| {
        f.set_options(&["Modern", "Legacy"]);
        f.set_default(1);
        f.set_value(f.default());
      }),
    )?;

    Ok(())
  }

  fn handle_command(
    &self,
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
      let pivot = PixelF32 {
        red: params.get(Params::RPivot)?.as_float_slider()?.value() as f32
          / 100.0,
        green: params.get(Params::GPivot)?.as_float_slider()?.value() as f32
          / 100.0,
        blue: params.get(Params::BPivot)?.as_float_slider()?.value() as f32
          / 100.0,
        alpha: params.get(Params::APivot)?.as_float_slider()?.value() as f32
          / 100.0,
      };

      Ok::<_, Error>((mode, gain, pivot))
    };
    match cmd {
      ae::Command::About => {
        out_data.set_return_msg(
"RGBA Contrast\rCopyright 2025 HanaYuki.\rThis allows you to adjust the contrast for each color.",
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

        let (mode, gain, pivot) = get_params()?;
        let curves = SigmoidCurves::new(gain, pivot);

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
                  *out_pixel = curves.apply(pixel.to_pixel32()).to_pixel8()
                }
                Mode::Legacy => {
                  *out_pixel =
                    pixel.to_pixel32().linear_contrast(gain, pivot).to_pixel8()
                }
              },
              (
                ae::GenericPixel::Pixel16(pixel),
                ae::GenericPixelMut::Pixel16(out_pixel),
              ) => match mode {
                Mode::Modern => {
                  *out_pixel = curves.apply(pixel.to_pixel32()).to_pixel16()
                }
                Mode::Legacy => {
                  *out_pixel =
                    pixel.to_pixel32().linear_contrast(gain, pivot).to_pixel16()
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
        } else {
          return Err(Error::InterruptCancel);
        }
      }
      ae::Command::SmartRender { extra } => {
        let cb = extra.callbacks();
        // 下にレイヤーのない調整レイヤーなどでは、描画対象が返らない。
        let Some(input_world) = cb.checkout_layer_pixels(0)? else {
          return Ok(());
        };
        let Some(mut output_world) = cb.checkout_output()? else {
          return cb.checkin_layer_pixels(0);
        };

        let progress_final = output_world.height() as _;

        let (mode, gain, pivot) = get_params()?;
        let curves = SigmoidCurves::new(gain, pivot);

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
                  *out_pixel = curves.apply(pixel.to_pixel32()).to_pixel8();
                }
                Mode::Legacy => {
                  *out_pixel =
                    pixel.to_pixel32().linear_contrast(gain, pivot).to_pixel8()
                }
              },
              (
                ae::GenericPixel::Pixel16(pixel),
                ae::GenericPixelMut::Pixel16(out_pixel),
              ) => match mode {
                Mode::Modern => {
                  *out_pixel = curves.apply(pixel.to_pixel32()).to_pixel16();
                }
                Mode::Legacy => {
                  *out_pixel =
                    pixel.to_pixel32().linear_contrast(gain, pivot).to_pixel16()
                }
              },
              (
                ae::GenericPixel::PixelF32(pixel),
                ae::GenericPixelMut::PixelF32(out_pixel),
              ) => match mode {
                Mode::Modern => {
                  *out_pixel = curves.apply(*pixel);
                }
                Mode::Legacy => *out_pixel = pixel.linear_contrast(gain, pivot),
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
