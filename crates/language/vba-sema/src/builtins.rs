//! The built-in part of the `VBA` library that is not a `vba-stdlib`
//! function: intrinsic constants, the `Err` and `Debug` objects, the
//! `Collection` class and the run-time provided functions (`CreateObject`,
//! `MsgBox`, ...).
//!
//! Provenance: names and values follow the documented VBA constants
//! (`vbCrLf`, `vbBinaryCompare`, `vbYes`, `VarType` constants, ...). Only
//! the commonly used subset is listed; names outside it resolve through the
//! injected [`TypeProvider`] or are reported.

use vba_value::{Variant, VbaString};

use crate::provider::{LibEvent, LibGlobal, LibMember, LibMemberKind, LibParam, TypeProvider};
use crate::types::{LibraryType, Type};

/// Name of the built-in library.
pub const VBA_LIBRARY: &str = "VBA";

/// `(name, value)` of the integer-valued constants.
const LONG_CONSTANTS: &[(&str, i32)] = &[
    // Compare
    ("vbBinaryCompare", 0),
    ("vbTextCompare", 1),
    ("vbDatabaseCompare", 2),
    // VarType
    ("vbEmpty", 0),
    ("vbNull", 1),
    ("vbInteger", 2),
    ("vbLong", 3),
    ("vbSingle", 4),
    ("vbDouble", 5),
    ("vbCurrency", 6),
    ("vbDate", 7),
    ("vbString", 8),
    ("vbObject", 9),
    ("vbError", 10),
    ("vbBoolean", 11),
    ("vbVariant", 12),
    ("vbDataObject", 13),
    ("vbDecimal", 14),
    ("vbByte", 17),
    ("vbLongLong", 20),
    ("vbUserDefinedType", 36),
    ("vbArray", 8192),
    // MsgBox
    ("vbOKOnly", 0),
    ("vbOKCancel", 1),
    ("vbAbortRetryIgnore", 2),
    ("vbYesNoCancel", 3),
    ("vbYesNo", 4),
    ("vbRetryCancel", 5),
    ("vbCritical", 16),
    ("vbQuestion", 32),
    ("vbExclamation", 48),
    ("vbInformation", 64),
    ("vbDefaultButton1", 0),
    ("vbDefaultButton2", 256),
    ("vbDefaultButton3", 512),
    ("vbDefaultButton4", 768),
    ("vbApplicationModal", 0),
    ("vbSystemModal", 4096),
    ("vbMsgBoxHelpButton", 16384),
    ("vbMsgBoxSetForeground", 65536),
    ("vbMsgBoxRight", 524288),
    ("vbMsgBoxRtlReading", 1048576),
    ("vbOK", 1),
    ("vbCancel", 2),
    ("vbAbort", 3),
    ("vbRetry", 4),
    ("vbIgnore", 5),
    ("vbYes", 6),
    ("vbNo", 7),
    // Dates
    ("vbUseSystemDayOfWeek", 0),
    ("vbSunday", 1),
    ("vbMonday", 2),
    ("vbTuesday", 3),
    ("vbWednesday", 4),
    ("vbThursday", 5),
    ("vbFriday", 6),
    ("vbSaturday", 7),
    ("vbUseSystem", 0),
    ("vbFirstJan1", 1),
    ("vbFirstFourDays", 2),
    ("vbFirstFullWeek", 3),
    ("vbGeneralDate", 0),
    ("vbLongDate", 1),
    ("vbShortDate", 2),
    ("vbLongTime", 3),
    ("vbShortTime", 4),
    // Strings
    ("vbUpperCase", 1),
    ("vbLowerCase", 2),
    ("vbProperCase", 3),
    ("vbWide", 4),
    ("vbNarrow", 8),
    ("vbKatakana", 16),
    ("vbHiragana", 32),
    ("vbSimplifiedChinese", 256),
    ("vbTraditionalChinese", 512),
    ("vbLinguisticCasing", 1024),
    ("vbUnicode", 64),
    ("vbFromUnicode", 128),
    ("vbUseDefault", -2),
    ("vbTrue", -1),
    ("vbFalse", 0),
    // Files
    ("vbNormal", 0),
    ("vbReadOnly", 1),
    ("vbHidden", 2),
    ("vbSystem", 4),
    ("vbVolume", 8),
    ("vbDirectory", 16),
    ("vbArchive", 32),
    ("vbAlias", 64),
    // CallType (CallByName)
    ("vbMethod", 1),
    ("vbGet", 2),
    ("vbLet", 4),
    ("vbSet", 8),
    // Predefined enums, ColorConstants, SystemColorConstants and KeyCodeConstants
    // (MS-VBAL 6.1.1, 6.1.2.1, 6.1.2.9, 6.1.2.12)
    ("vb3DDKShadow", -2147483627),
    ("vb3DFace", -2147483633),
    ("vb3DHighlight", -2147483628),
    ("vb3DLight", -2147483626),
    ("vb3Dshadow", -2147483632),
    ("vbActiveBorder", -2147483638),
    ("vbActiveTitleBar", -2147483646),
    ("vbAppTaskManager", 3),
    ("vbAppWindows", 2),
    ("vbApplicationWorkspace", -2147483636),
    ("vbBlack", 0),
    ("vbBlue", 16711680),
    ("vbButtonFace", -2147483633),
    ("vbButtonShadow", -2147483632),
    ("vbButtonText", -2147483630),
    ("vbCalGreg", 0),
    ("vbCalHijri", 1),
    ("vbCyan", 16776960),
    ("vbDesktop", -2147483647),
    ("vbFormCode", 1),
    ("vbFormControlMenu", 0),
    ("vbFormMDIForm", 4),
    ("vbGrayText", -2147483631),
    ("vbGreen", 65280),
    ("vbHide", 0),
    ("vbHighlight", -2147483635),
    ("vbHighlightText", -2147483634),
    ("vbIMEAlphaDbl", 7),
    ("vbIMEAlphaSng", 8),
    ("vbIMEDisable", 3),
    ("vbIMEHiragana", 4),
    ("vbIMEKatakanaDbl", 5),
    ("vbIMEKatakanaSng", 6),
    ("vbIMEModeAlpha", 8),
    ("vbIMEModeAlphaFull", 7),
    ("vbIMEModeDisable", 3),
    ("vbIMEModeHangul", 10),
    ("vbIMEModeHangulFull", 9),
    ("vbIMEModeHiragana", 4),
    ("vbIMEModeKatakana", 5),
    ("vbIMEModeKatakanaHalf", 6),
    ("vbIMEModeNoControl", 0),
    ("vbIMEModeOff", 2),
    ("vbIMEModeOn", 1),
    ("vbIMENoOp", 0),
    ("vbIMEOff", 2),
    ("vbIMEOn", 1),
    ("vbInactiveBorder", -2147483637),
    ("vbInactiveCaptionText", -2147483629),
    ("vbInactiveTitleBar", -2147483645),
    ("vbInfoBackground", -2147483624),
    ("vbInfoText", -2147483625),
    ("vbKey0", 48),
    ("vbKey1", 49),
    ("vbKey2", 50),
    ("vbKey3", 51),
    ("vbKey4", 52),
    ("vbKey5", 53),
    ("vbKey6", 54),
    ("vbKey7", 55),
    ("vbKey8", 56),
    ("vbKey9", 57),
    ("vbKeyA", 65),
    ("vbKeyAdd", 107),
    ("vbKeyB", 66),
    ("vbKeyBack", 8),
    ("vbKeyC", 67),
    ("vbKeyCancel", 3),
    ("vbKeyCapital", 20),
    ("vbKeyClear", 12),
    ("vbKeyControl", 17),
    ("vbKeyD", 68),
    ("vbKeyDecimal", 110),
    ("vbKeyDelete", 46),
    ("vbKeyDivide", 111),
    ("vbKeyDown", 40),
    ("vbKeyE", 69),
    ("vbKeyEnd", 35),
    ("vbKeyEscape", 27),
    ("vbKeyExecute", 43),
    ("vbKeyF", 70),
    ("vbKeyF1", 112),
    ("vbKeyF10", 121),
    ("vbKeyF11", 122),
    ("vbKeyF12", 123),
    ("vbKeyF13", 124),
    ("vbKeyF14", 125),
    ("vbKeyF15", 126),
    ("vbKeyF16", 127),
    ("vbKeyF2", 113),
    ("vbKeyF3", 114),
    ("vbKeyF4", 115),
    ("vbKeyF5", 116),
    ("vbKeyF6", 117),
    ("vbKeyF7", 118),
    ("vbKeyF8", 119),
    ("vbKeyF9", 120),
    ("vbKeyG", 71),
    ("vbKeyH", 72),
    ("vbKeyHelp", 47),
    ("vbKeyHome", 36),
    ("vbKeyI", 73),
    ("vbKeyInsert", 45),
    ("vbKeyJ", 74),
    ("vbKeyK", 75),
    ("vbKeyL", 76),
    ("vbKeyLButton", 1),
    ("vbKeyLeft", 37),
    ("vbKeyM", 77),
    ("vbKeyMButton", 4),
    ("vbKeyMenu", 18),
    ("vbKeyMultiply", 106),
    ("vbKeyN", 78),
    ("vbKeyNumlock", 144),
    ("vbKeyNumpad0", 96),
    ("vbKeyNumpad1", 97),
    ("vbKeyNumpad2", 98),
    ("vbKeyNumpad3", 99),
    ("vbKeyNumpad4", 100),
    ("vbKeyNumpad5", 101),
    ("vbKeyNumpad6", 102),
    ("vbKeyNumpad7", 103),
    ("vbKeyNumpad8", 104),
    ("vbKeyNumpad9", 105),
    ("vbKeyO", 79),
    ("vbKeyP", 80),
    ("vbKeyPageDown", 34),
    ("vbKeyPageUp", 33),
    ("vbKeyPause", 19),
    ("vbKeyPrint", 42),
    ("vbKeyQ", 81),
    ("vbKeyR", 82),
    ("vbKeyRButton", 2),
    ("vbKeyReturn", 13),
    ("vbKeyRight", 39),
    ("vbKeyS", 83),
    ("vbKeySelect", 41),
    ("vbKeySeparator", 108),
    ("vbKeyShift", 16),
    ("vbKeySnapshot", 44),
    ("vbKeySpace", 32),
    ("vbKeySubtract", 109),
    ("vbKeyT", 84),
    ("vbKeyTab", 9),
    ("vbKeyU", 85),
    ("vbKeyUp", 38),
    ("vbKeyV", 86),
    ("vbKeyW", 87),
    ("vbKeyX", 88),
    ("vbKeyY", 89),
    ("vbKeyZ", 90),
    ("vbMagenta", 16711935),
    ("vbMaximizedFocus", 3),
    ("vbMenuBar", -2147483644),
    ("vbMenuText", -2147483641),
    ("vbMinimizedFocus", 2),
    ("vbMinimizedNoFocus", 6),
    ("vbModal", 1),
    ("vbModeless", 0),
    ("vbNormalFocus", 1),
    ("vbNormalNoFocus", 4),
    ("vbRed", 255),
    ("vbScrollBars", -2147483648),
    ("vbTitleBarText", -2147483639),
    ("vbWhite", 16777215),
    ("vbWindowBackground", -2147483643),
    ("vbWindowFrame", -2147483642),
    ("vbWindowText", -2147483640),
    ("vbYellow", 65535),
    // Misc
    ("vbObjectError", -2_147_221_504),
];

const STRING_CONSTANTS: &[(&str, &str)] = &[
    ("vbCrLf", "\r\n"),
    ("vbNewLine", "\r\n"),
    ("vbCr", "\r"),
    ("vbLf", "\n"),
    ("vbTab", "\t"),
    ("vbBack", "\u{8}"),
    ("vbFormFeed", "\u{c}"),
    ("vbVerticalTab", "\u{b}"),
    ("vbNullChar", "\0"),
    ("vbNullString", ""),
];

fn p(name: &str, ty: Type) -> LibParam {
    LibParam::required(name, ty)
}

fn o(name: &str, ty: Type) -> LibParam {
    LibParam::optional(name, ty)
}

fn lib(name: &str) -> LibraryType {
    LibraryType {
        library: VBA_LIBRARY.to_owned(),
        name: name.to_owned(),
    }
}

/// A `stdole` interface type (`IUnknown`, `IDispatch`).
pub fn stdole_type(name: &str) -> LibraryType {
    LibraryType {
        library: "stdole".to_owned(),
        name: name.to_owned(),
    }
}

/// The type of the `Err` object.
pub fn err_object_type() -> LibraryType {
    lib("ErrObject")
}

/// The type of the `Collection` class.
pub fn collection_type() -> LibraryType {
    lib("Collection")
}

/// Run-time provided functions that are not in the `vba-stdlib` registry.
pub fn runtime_function(name: &str) -> Option<LibMember> {
    let n = name.to_ascii_lowercase();
    Some(match n.as_str() {
        "createobject" => LibMember::function(
            "CreateObject",
            vec![p("Class", Type::String), o("ServerName", Type::String)],
            Type::Object,
        ),
        "getobject" => LibMember::function(
            "GetObject",
            vec![o("PathName", Type::Variant), o("Class", Type::Variant)],
            Type::Object,
        ),
        "msgbox" => LibMember::function(
            "MsgBox",
            vec![
                p("Prompt", Type::Variant),
                o("Buttons", Type::Long),
                o("Title", Type::Variant),
                o("HelpFile", Type::Variant),
                o("Context", Type::Variant),
            ],
            Type::Long,
        ),
        "inputbox" => LibMember::function(
            "InputBox",
            vec![
                p("Prompt", Type::String),
                o("Title", Type::Variant),
                o("Default", Type::Variant),
                o("XPos", Type::Variant),
                o("YPos", Type::Variant),
                o("HelpFile", Type::Variant),
                o("Context", Type::Variant),
            ],
            Type::String,
        ),
        "doevents" => LibMember::function("DoEvents", vec![], Type::Integer),
        "beep" => LibMember::sub("Beep", vec![]),
        "erl" => LibMember::function("Erl", vec![], Type::Long),
        "command" => LibMember::function("Command", vec![], Type::Variant),
        "ismissing" => LibMember::function(
            "IsMissing",
            vec![p("ArgName", Type::Variant)],
            Type::Boolean,
        ),
        "shell" => LibMember::function(
            "Shell",
            vec![p("PathName", Type::String), o("WindowStyle", Type::Integer)],
            Type::Double,
        ),
        // File system functions and statements (`VBA.FileSystem`).
        "freefile" => LibMember::function(
            "FreeFile",
            vec![o("RangeNumber", Type::Variant)],
            Type::Integer,
        ),
        "eof" => LibMember::function("EOF", vec![p("FileNumber", Type::Integer)], Type::Boolean),
        "lof" => LibMember::function("LOF", vec![p("FileNumber", Type::Integer)], Type::Long),
        "loc" => LibMember::function("Loc", vec![p("FileNumber", Type::Integer)], Type::Long),
        "seek" => LibMember::function("Seek", vec![p("FileNumber", Type::Integer)], Type::Long),
        "input" => LibMember::function(
            "Input",
            vec![p("Number", Type::Long), p("FileNumber", Type::Integer)],
            Type::String,
        ),
        "inputb" => LibMember::function(
            "InputB",
            vec![p("Number", Type::Long), p("FileNumber", Type::Integer)],
            Type::String,
        ),
        "fileattr" => LibMember::function(
            "FileAttr",
            vec![
                p("FileNumber", Type::Integer),
                o("ReturnType", Type::Integer),
            ],
            Type::Long,
        ),
        "reset" => LibMember::sub("Reset", vec![]),
        "callbyname" => LibMember::function(
            "CallByName",
            vec![
                p("Object", Type::Variant),
                p("ProcName", Type::String),
                p("CallType", Type::Long),
                LibParam {
                    param_array: true,
                    optional: true,
                    ..LibParam::required("Args", Type::Variant)
                },
            ],
            Type::Variant,
        ),
        // Settings (`VBA.Interaction`); the store is a host service.
        "savesetting" => LibMember::sub(
            "SaveSetting",
            vec![
                p("AppName", Type::String),
                p("Section", Type::String),
                p("Key", Type::String),
                p("Setting", Type::String),
            ],
        ),
        "getsetting" => LibMember::function(
            "GetSetting",
            vec![
                p("AppName", Type::String),
                p("Section", Type::String),
                p("Key", Type::String),
                o("Default", Type::Variant),
            ],
            Type::String,
        ),
        "getallsettings" => LibMember::function(
            "GetAllSettings",
            vec![p("AppName", Type::String), p("Section", Type::String)],
            Type::Variant,
        ),
        "deletesetting" => LibMember::sub(
            "DeleteSetting",
            vec![
                p("AppName", Type::String),
                o("Section", Type::String),
                o("Key", Type::String),
            ],
        ),
        // UI automation: refused by the host unless granted, and then still
        // only forwarded to a host hook.
        "sendkeys" => LibMember::sub(
            "SendKeys",
            vec![p("String", Type::String), o("Wait", Type::Boolean)],
        ),
        "appactivate" => LibMember::sub(
            "AppActivate",
            vec![p("Title", Type::Variant), o("Wait", Type::Boolean)],
        ),
        "filelen" => LibMember::function("FileLen", vec![p("PathName", Type::String)], Type::Long),
        "filedatetime" => LibMember::function(
            "FileDateTime",
            vec![p("PathName", Type::String)],
            Type::Date,
        ),
        "getattr" => {
            LibMember::function("GetAttr", vec![p("PathName", Type::String)], Type::Integer)
        }
        "curdir" => LibMember::function("CurDir", vec![o("Drive", Type::Variant)], Type::Variant),
        "dir" => LibMember::function(
            "Dir",
            vec![o("PathName", Type::Variant), o("Attributes", Type::Long)],
            Type::String,
        ),
        "setattr" => LibMember::sub(
            "SetAttr",
            vec![p("PathName", Type::String), p("Attributes", Type::Integer)],
        ),
        "kill" => LibMember::sub("Kill", vec![p("PathName", Type::Variant)]),
        "mkdir" => LibMember::sub("MkDir", vec![p("Path", Type::String)]),
        "rmdir" => LibMember::sub("RmDir", vec![p("Path", Type::String)]),
        "chdir" => LibMember::sub("ChDir", vec![p("Path", Type::String)]),
        "chdrive" => LibMember::sub("ChDrive", vec![p("Drive", Type::Variant)]),
        "filecopy" => LibMember::sub(
            "FileCopy",
            vec![p("Source", Type::String), p("Destination", Type::String)],
        ),
        _ => return None,
    })
}

/// The built-in members of `VBA` library types.
fn builtin_member(ty: &LibraryType, name: &str) -> Option<LibMember> {
    let n = name.to_ascii_lowercase();
    let v = Type::Variant;
    match ty.name.as_str() {
        "ErrObject" => Some(match n.as_str() {
            "number" => LibMember::property("Number", Type::Long).default_member(),
            "description" => LibMember::property("Description", Type::String),
            "source" => LibMember::property("Source", Type::String),
            "helpfile" => LibMember::property("HelpFile", Type::String),
            "helpcontext" => LibMember::property("HelpContext", Type::Long),
            "lastdllerror" => LibMember {
                kind: LibMemberKind::PropertyGet,
                ..LibMember::property("LastDllError", Type::Long)
            },
            "clear" => LibMember::sub("Clear", vec![]),
            "raise" => LibMember::sub(
                "Raise",
                vec![
                    p("Number", Type::Long),
                    o("Source", v.clone()),
                    o("Description", v.clone()),
                    o("HelpFile", v.clone()),
                    o("HelpContext", v),
                ],
            ),
            _ => return None,
        }),
        "DebugObject" => Some(match n.as_str() {
            "assert" => LibMember::sub("Assert", vec![p("Condition", Type::Boolean)]),
            "print" => LibMember::sub("Print", vec![]),
            _ => return None,
        }),
        "Collection" => Some(match n.as_str() {
            "add" => LibMember::sub(
                "Add",
                vec![
                    p("Item", v.clone()),
                    o("Key", Type::String),
                    o("Before", v.clone()),
                    o("After", v),
                ],
            ),
            "count" => LibMember {
                kind: LibMemberKind::PropertyGet,
                ..LibMember::property("Count", Type::Long)
            },
            "item" => LibMember::function("Item", vec![p("Index", Type::Variant)], Type::Variant)
                .default_member(),
            "remove" => LibMember::sub("Remove", vec![p("Index", Type::Variant)]),
            "_newenum" => {
                LibMember::function("_NewEnum", vec![], Type::Library(stdole_type("IUnknown")))
            }
            _ => return None,
        }),
        _ => None,
    }
}

/// The built-in library as a provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltinLibrary;

/// The enumeration types of the VBA library. Their members are the `vb...`
/// constants; a variable of such a type holds a `Long`.
const VBA_ENUM_TYPES: &[&str] = &[
    "VbAppWinStyle",
    "VbCalendar",
    "VbCallType",
    "VbCompareMethod",
    "VbDateTimeFormat",
    "VbDayOfWeek",
    "VbFileAttribute",
    "VbFirstWeekOfYear",
    "VbIMEStatus",
    "VbMsgBoxResult",
    "VbMsgBoxStyle",
    "VbQueryClose",
    "VbStrConv",
    "VbTriState",
    "VbVarType",
];

impl TypeProvider for BuiltinLibrary {
    fn resolve_declared(&self, qualifier: Option<&str>, name: &str) -> Option<Type> {
        let vba_scope = qualifier.is_none_or(|q| q.eq_ignore_ascii_case(VBA_LIBRARY));
        if vba_scope
            && VBA_ENUM_TYPES
                .iter()
                .any(|e| e.eq_ignore_ascii_case(name))
        {
            return Some(Type::Long);
        }
        self.resolve_type(qualifier, name).map(Type::Library)
    }

    fn resolve_type(&self, qualifier: Option<&str>, name: &str) -> Option<LibraryType> {
        let lower = name.to_ascii_lowercase();
        // `stdole` interfaces are visible to every VBA project.
        if qualifier.is_none_or(|q| q.eq_ignore_ascii_case("stdole")) {
            match lower.as_str() {
                "iunknown" => return Some(stdole_type("IUnknown")),
                "idispatch" => return Some(stdole_type("IDispatch")),
                _ => {}
            }
        }
        if let Some(q) = qualifier
            && !q.eq_ignore_ascii_case(VBA_LIBRARY)
        {
            return None;
        }
        match lower.as_str() {
            "collection" => Some(collection_type()),
            "errobject" => Some(err_object_type()),
            _ => None,
        }
    }

    fn member(&self, ty: &LibraryType, name: &str) -> Option<LibMember> {
        if ty.library != VBA_LIBRARY {
            return None;
        }
        builtin_member(ty, name)
    }

    fn default_member(&self, ty: &LibraryType) -> Option<LibMember> {
        if ty.library != VBA_LIBRARY {
            return None;
        }
        ["Number", "Item"]
            .iter()
            .filter_map(|n| builtin_member(ty, n))
            .find(|m| m.is_default)
    }

    fn describes_members(&self, ty: &LibraryType) -> bool {
        ty.library == VBA_LIBRARY
            && matches!(ty.name.as_str(), "ErrObject" | "DebugObject" | "Collection")
    }

    fn events(&self, ty: &LibraryType) -> Option<Vec<LibEvent>> {
        (ty.library == VBA_LIBRARY).then(Vec::new)
    }

    fn global(&self, name: &str) -> Option<LibGlobal> {
        let n = name.to_ascii_lowercase();
        if n == "err" {
            return Some(LibGlobal::Object(Type::Library(err_object_type())));
        }
        if n == "debug" {
            return Some(LibGlobal::Object(Type::Library(lib("DebugObject"))));
        }
        if let Some((_, v)) = LONG_CONSTANTS
            .iter()
            .find(|(k, _)| k.to_ascii_lowercase() == n && !k.ends_with('_'))
        {
            return Some(LibGlobal::Constant(Variant::Long(*v), Type::Long));
        }
        if let Some((_, v)) = STRING_CONSTANTS
            .iter()
            .find(|(k, _)| k.to_ascii_lowercase() == n)
        {
            return Some(LibGlobal::Constant(
                Variant::String(VbaString::from(*v)),
                Type::String,
            ));
        }
        runtime_function(name).map(LibGlobal::Function)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strconv_flag_constants_resolve_as_long() {
        for (name, value) in [
            ("vbSimplifiedChinese", 256),
            ("vbTraditionalChinese", 512),
            ("vbLinguisticCasing", 1024),
        ] {
            for n in [name.to_owned(), name.to_ascii_lowercase()] {
                match BuiltinLibrary.global(&n) {
                    Some(LibGlobal::Constant(Variant::Long(v), Type::Long)) => {
                        assert_eq!(v, value, "{n}");
                    }
                    _ => panic!("{n} is not a Long constant"),
                }
            }
        }
    }
}
