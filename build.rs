use pipl::*;
use std::env;

const PF_PLUG_IN_VERSION: u16 = 13;
const PF_PLUG_IN_SUBVERS: u16 = 28;

fn cargo_version_component(name: &str, maximum: u32) -> u32 {
  let value =
    env::var(name).unwrap_or_else(|_| panic!("{name} が設定されていません"));
  let component = value
    .parse::<u32>()
    .unwrap_or_else(|_| panic!("{name} は整数ではありません: {value}"));
  assert!(
    component <= maximum,
    "{name} は {maximum} 以下である必要があります: {component}"
  );
  component
}

fn cargo_release_stage() -> Stage {
  match env::var("CARGO_PKG_VERSION_PRE")
    .unwrap_or_default()
    .as_str()
  {
    "" => Stage::Release,
    pre if pre.starts_with("alpha") => Stage::Alpha,
    pre if pre.starts_with("beta") || pre.starts_with("rc") => Stage::Beta,
    _ => Stage::Develop,
  }
}

#[rustfmt::skip]
fn main() {
  let version = cargo_version_component("CARGO_PKG_VERSION_MAJOR", 127);
  let subversion = cargo_version_component("CARGO_PKG_VERSION_MINOR", 15);
  let bugversion = cargo_version_component("CARGO_PKG_VERSION_PATCH", 15);

  pipl::plugin_build(vec![
    Property::Kind(PIPLType::AEEffect),
    Property::Name("RGBA Contrast"),
    Property::Category("hanayuki"),

    #[cfg(target_os = "windows")]
    Property::CodeWin64X86("EffectMain"),
    #[cfg(target_os = "macos")]
    Property::CodeMacIntel64("EffectMain"),
    #[cfg(target_os = "macos")]
    Property::CodeMacARM64("EffectMain"),

    Property::AE_PiPL_Version { major: 2, minor: 0 },
    Property::AE_Effect_Spec_Version {
      major: PF_PLUG_IN_VERSION,
      minor: PF_PLUG_IN_SUBVERS,
    },
    Property::AE_Effect_Version {
      version,
      subversion,
      bugversion,
      stage: cargo_release_stage(),
      build: 1,
    },

    Property::AE_Effect_Info_Flags(0),
    Property::AE_Effect_Global_OutFlags(
      OutFlags::SendUpdateParamsUI
        | OutFlags::PixIndependent
        | OutFlags::UseOutputExtent
        | OutFlags::DeepColorAware,
    ),
    Property::AE_Effect_Global_OutFlags_2(
      OutFlags2::ParamGroupStartCollapsedFlag
        | OutFlags2::RevealsZeroAlpha
        | OutFlags2::SupportsThreadedRendering
        | OutFlags2::FloatColorAware
        | OutFlags2::SupportsSmartRender,
    ),

    Property::AE_Effect_Match_Name("HANAYUKI RGBA Contrast"),
    Property::AE_Reserved_Info(8),
    Property::AE_Effect_Support_URL(""),
  ])
}
