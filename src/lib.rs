#![allow(
  clippy::drop_non_drop,
  clippy::question_mark,
  reason = "after-effects の外部マクロ展開で発生するため"
)]

use after_effects as ae;
mod color;
mod contrast;
mod pixels;
mod premiere;
use color::{Adjustment, Space, Transfer};
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
  ColorSpace,
  InputTransfer,
  Gamma1,
  Gamma2,
  Gamma3,
  GammaAlpha,
  HueOrigin,
  SeparateAlpha,
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
  fn can_load(_host_name: &str, _host_version: &str) -> bool {
    true
  }

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

    params.add_with_flags(
      Params::ColorSpace,
      "Color Space",
      ae::PopupDef::setup(|f| {
        f.set_options(&["RGB", "HSL", "HSV", "YUV (BT.709)", "OKLab", "OKLch"]);
        f.set_default(1);
        f.set_value(1);
      }),
      ae::ParamFlag::CANNOT_TIME_VARY,
      ae::ParamUIFlags::NONE,
    )?;
    params.add_with_flags(
      Params::InputTransfer,
      "Input RGB",
      ae::PopupDef::setup(|f| {
        f.set_options(&[
          "sRGB",
          "Rec.709 Gamma 2.4",
          "Rec.709",
          "Linear sRGB / Rec.709",
        ]);
        f.set_default(1);
        f.set_value(1);
      }),
      ae::ParamFlag::CANNOT_TIME_VARY,
      ae::ParamUIFlags::NONE,
    )?;
    for (id, name) in [
      (Params::Gamma1, "Axis 1 Gamma"),
      (Params::Gamma2, "Axis 2 Gamma"),
      (Params::Gamma3, "Axis 3 Gamma"),
      (Params::GammaAlpha, "Alpha Gamma"),
    ] {
      params.add(
        id,
        name,
        ae::FloatSliderDef::setup(|f| {
          f.set_slider_min(0.25);
          f.set_slider_max(4.0);
          f.set_valid_min(0.1);
          f.set_valid_max(10.0);
          f.set_default(1.0);
          f.set_precision(2);
        }),
      )?;
    }
    params.add(
      Params::HueOrigin,
      "Hue Gamma Origin",
      ae::FloatSliderDef::setup(|f| {
        f.set_slider_min(0.0);
        f.set_slider_max(360.0);
        f.set_valid_min(0.0);
        f.set_valid_max(360.0);
        f.set_default(0.0);
        f.set_precision(2);
      }),
    )?;
    params.add_with_flags(
      Params::SeparateAlpha,
      "Separate Color and Alpha",
      ae::CheckBoxDef::setup(|f| {
        f.set_default(true);
        f.set_value(false);
      }),
      ae::ParamFlag::CANNOT_TIME_VARY
        | ae::ParamFlag::USE_VALUE_FOR_OLD_PROJECTS,
      ae::ParamUIFlags::NONE,
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

      let space = match params.get(Params::ColorSpace)?.as_popup()?.value() {
        1 => Space::Rgb,
        2 => Space::Hsl,
        3 => Space::Hsv,
        4 => Space::Yuv,
        5 => Space::Oklab,
        6 => Space::Oklch,
        _ => return Err(Error::BadCallbackParameter),
      };
      let transfer =
        match params.get(Params::InputTransfer)?.as_popup()?.value() {
          1 => Transfer::Srgb,
          2 => Transfer::Rec709Display,
          3 => Transfer::Rec709,
          4 => Transfer::Linear,
          _ => return Err(Error::BadCallbackParameter),
        };
      let gamma = [
        params.get(Params::Gamma1)?.as_float_slider()?.value() as f32,
        params.get(Params::Gamma2)?.as_float_slider()?.value() as f32,
        params.get(Params::Gamma3)?.as_float_slider()?.value() as f32,
        params.get(Params::GammaAlpha)?.as_float_slider()?.value() as f32,
      ];
      let contrast = [
        params.get(Params::Red)?.as_float_slider()?.value() as f32,
        params.get(Params::Green)?.as_float_slider()?.value() as f32,
        params.get(Params::Blue)?.as_float_slider()?.value() as f32,
        params.get(Params::Alpha)?.as_float_slider()?.value() as f32,
      ];
      let pivots = [pivot.red, pivot.green, pivot.blue, pivot.alpha];
      Ok::<_, Error>(Adjustment::new(
        space,
        transfer,
        mode == Mode::Modern,
        contrast,
        pivots,
        gamma,
        params.get(Params::HueOrigin)?.as_float_slider()?.value() as f32,
        params.get(Params::SeparateAlpha)?.as_checkbox()?.value(),
        !in_data.is_premiere(),
      ))
    };
    match cmd {
      ae::Command::About => {
        out_data.set_return_msg(
"RGBA Contrast\rCopyright 2025 HanaYuki.\rThis allows you to adjust the contrast for each color.",
);
      }
      ae::Command::GlobalSetup => {
        out_data.set_out_flag(ae::OutFlags::SendUpdateParamsUi, true);
        out_data.set_out_flag2(ae::OutFlags2::RevealsZeroAlpha, true);
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
      ae::Command::UpdateParamsUi => {
        let selected_space =
          params.get(Params::ColorSpace)?.as_popup()?.value();
        let labels = match selected_space {
          1 => ["Red", "Green", "Blue"],
          2 => ["H", "S", "L"],
          3 => ["H", "S", "V"],
          4 => ["Y", "U", "V"],
          5 => ["L", "a", "b"],
          6 => ["L", "C", "h"],
          _ => return Err(Error::BadCallbackParameter),
        };
        for (i, (gain, pivot, gamma)) in [
          (Params::Red, Params::RPivot, Params::Gamma1),
          (Params::Green, Params::GPivot, Params::Gamma2),
          (Params::Blue, Params::BPivot, Params::Gamma3),
        ]
        .into_iter()
        .enumerate()
        {
          for (id, label) in [
            (
              gain,
              if selected_space == 1 {
                labels[i].to_string()
              } else {
                format!("{} Contrast", labels[i])
              },
            ),
            (pivot, format!("{} Pivot", labels[i])),
            (gamma, format!("{} Gamma", labels[i])),
          ] {
            let mut p = params.get_mut(id)?;
            p.set_name(&label);
            p.update_param_ui()?;
          }
        }
      }
      ae::Command::Render {
        in_layer,
        mut out_layer,
      } => {
        let progress_final = out_layer.height() as _;

        let adjustment = get_params()?;
        if in_data.is_premiere() {
          return premiere::render(&in_layer, &mut out_layer, &adjustment);
        }

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
              ) => {
                *out_pixel = adjustment.apply(pixel.to_pixel32()).to_pixel8()
              }
              (
                ae::GenericPixel::Pixel16(pixel),
                ae::GenericPixelMut::Pixel16(out_pixel),
              ) => {
                *out_pixel = adjustment.apply(pixel.to_pixel32()).to_pixel16()
              }
              (
                ae::GenericPixel::PixelF32(pixel),
                ae::GenericPixelMut::PixelF32(out_pixel),
              ) => *out_pixel = adjustment.apply(*pixel),
              _ => return Err(Error::BadCallbackParameter),
            }
            Ok(())
          },
        )?;
      }
      ae::Command::SmartPreRender { mut extra } => {
        let mut req = extra.output_request();
        // アルファ操作で透明な画素を可視化する場合に備える。
        req.preserve_rgb_of_zero_alpha = 1;

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
        let input_world = cb.checkout_layer_pixels(0)?;
        let mut output_world = cb.checkout_output()?;

        let progress_final = output_world.height() as _;

        let adjustment = get_params()?;
        if in_data.is_premiere() {
          premiere::render(&input_world, &mut output_world, &adjustment)?;
          cb.checkin_layer_pixels(0)?;
          return Ok(());
        }

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
              ) => {
                *out_pixel = adjustment.apply(pixel.to_pixel32()).to_pixel8()
              }
              (
                ae::GenericPixel::Pixel16(pixel),
                ae::GenericPixelMut::Pixel16(out_pixel),
              ) => {
                *out_pixel = adjustment.apply(pixel.to_pixel32()).to_pixel16()
              }
              (
                ae::GenericPixel::PixelF32(pixel),
                ae::GenericPixelMut::PixelF32(out_pixel),
              ) => *out_pixel = adjustment.apply(*pixel),
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
