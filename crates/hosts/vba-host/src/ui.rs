//! Message and input boxes.

use crate::host::HostResult;
use vba_automation::ErrInfo;

/// Arguments of `MsgBox`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsgBoxRequest {
    /// The prompt text.
    pub prompt: String,
    /// The `Buttons` argument (`vbOKOnly` = 0 ...).
    pub buttons: i32,
    /// The title.
    pub title: String,
}

/// Arguments of `InputBox`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBoxRequest {
    /// The prompt text.
    pub prompt: String,
    /// The title.
    pub title: String,
    /// The default text.
    pub default: String,
}

/// User-interface hooks. Both are optional features of a host.
pub trait UiHooks {
    /// `MsgBox`: returns the `vbOK`-style button constant.
    fn msg_box(&self, req: &MsgBoxRequest) -> HostResult<i32>;

    /// `InputBox`: `Ok(None)` means the user cancelled (VBA returns `""`).
    fn input_box(&self, req: &InputBoxRequest) -> HostResult<Option<String>>;
}

/// The default UI of a headless host: every call fails with error 70
/// ("Permission denied") and a description saying no UI exists. It never
/// invents an answer.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeadlessUi;

impl HeadlessUi {
    fn unavailable(what: &str) -> ErrInfo {
        ErrInfo::new(70, format!("{what} is not available: the host is headless"))
            .with_source("vba-host: headless UI")
    }
}

impl UiHooks for HeadlessUi {
    fn msg_box(&self, _: &MsgBoxRequest) -> HostResult<i32> {
        Err(Self::unavailable("MsgBox"))
    }

    fn input_box(&self, _: &InputBoxRequest) -> HostResult<Option<String>> {
        Err(Self::unavailable("InputBox"))
    }
}
