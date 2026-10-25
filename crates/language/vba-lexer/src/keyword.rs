//! Reserved identifiers (MS-VBAL §3.3.5.2) and their availability per
//! language generation.

use vba_profile::{Feature, LanguageGeneration};

/// Primary category of a [`Keyword`], following the spec's grammar rules.
///
/// Some words belong to several spec categories (`Case`, `Else`, `Input` and
/// `Write` are both statement and marker keywords, `Date` and `String` are both
/// reserved names and type identifiers); the first-listed category is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordClass {
    /// `statement-keyword`.
    Statement,
    /// `marker-keyword`.
    Marker,
    /// `operator-identifier`.
    Operator,
    /// `reserved-name`.
    ReservedName,
    /// `special-form`.
    SpecialForm,
    /// `reserved-type-identifier`.
    ReservedType,
    /// `literal-identifier` (`True`, `False`, `Nothing`, `Empty`, `Null`).
    Literal,
    /// `reserved-for-implementation-use` (`Attribute`, `VB_*`, `LINEINPUT`).
    ImplementationUse,
    /// `future-reserved`.
    FutureReserved,
    /// A word the lexer treats as reserved only in some generations and that
    /// the specification does not list (`PtrSafe`).
    Contextual,
}

// Keyword table, MS-VBAL §3.3.5.2.
macro_rules! keywords {
    ($($variant:ident => [$($pat:literal),+] $class:ident,)*) => {
        /// A VBA reserved identifier (MS-VBAL §3.3.5.2), plus the contextual
        /// `PtrSafe`. Matching is ASCII case-insensitive; the original spelling
        /// is always kept in the token text.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Keyword {
            $(
                #[doc = concat!("`", stringify!($variant), "`")]
                $variant,
            )*
        }

        impl Keyword {
            /// Every keyword.
            pub const ALL: &'static [Keyword] = &[$(Keyword::$variant,)*];

            /// Looks up a keyword from already lower-cased ASCII bytes.
            pub fn from_lower(lower: &[u8]) -> Option<Keyword> {
                match lower {
                    $($($pat)|+ => Some(Keyword::$variant),)*
                    _ => None,
                }
            }

            /// The primary category of the keyword.
            pub const fn class(self) -> KeywordClass {
                match self {
                    $(Keyword::$variant => KeywordClass::$class,)*
                }
            }
        }
    };
}

keywords! {
    Call => [b"call"] Statement,
    Case => [b"case"] Statement,
    Close => [b"close"] Statement,
    Const => [b"const"] Statement,
    Declare => [b"declare"] Statement,
    DefBool => [b"defbool"] Statement,
    DefByte => [b"defbyte"] Statement,
    DefCur => [b"defcur"] Statement,
    DefDate => [b"defdate"] Statement,
    DefDbl => [b"defdbl"] Statement,
    DefInt => [b"defint"] Statement,
    DefLng => [b"deflng"] Statement,
    DefLngLng => [b"deflnglng"] Statement,
    DefLngPtr => [b"deflngptr"] Statement,
    DefObj => [b"defobj"] Statement,
    DefSng => [b"defsng"] Statement,
    DefStr => [b"defstr"] Statement,
    DefVar => [b"defvar"] Statement,
    Dim => [b"dim"] Statement,
    Do => [b"do"] Statement,
    Else => [b"else"] Statement,
    ElseIf => [b"elseif"] Statement,
    End => [b"end"] Statement,
    EndIf => [b"endif"] Statement,
    Enum => [b"enum"] Statement,
    Erase => [b"erase"] Statement,
    Event => [b"event"] Statement,
    Exit => [b"exit"] Statement,
    For => [b"for"] Statement,
    Friend => [b"friend"] Statement,
    Function => [b"function"] Statement,
    Get => [b"get"] Statement,
    Global => [b"global"] Statement,
    GoSub => [b"gosub"] Statement,
    GoTo => [b"goto"] Statement,
    If => [b"if"] Statement,
    Implements => [b"implements"] Statement,
    Input => [b"input"] Statement,
    Let => [b"let"] Statement,
    Lock => [b"lock"] Statement,
    Loop => [b"loop"] Statement,
    LSet => [b"lset"] Statement,
    Next => [b"next"] Statement,
    On => [b"on"] Statement,
    Open => [b"open"] Statement,
    Option => [b"option"] Statement,
    Print => [b"print"] Statement,
    Private => [b"private"] Statement,
    Public => [b"public"] Statement,
    Put => [b"put"] Statement,
    RaiseEvent => [b"raiseevent"] Statement,
    ReDim => [b"redim"] Statement,
    Resume => [b"resume"] Statement,
    Return => [b"return"] Statement,
    RSet => [b"rset"] Statement,
    Seek => [b"seek"] Statement,
    Select => [b"select"] Statement,
    Set => [b"set"] Statement,
    Static => [b"static"] Statement,
    Stop => [b"stop"] Statement,
    Sub => [b"sub"] Statement,
    Type => [b"type"] Statement,
    Unlock => [b"unlock"] Statement,
    Wend => [b"wend"] Statement,
    While => [b"while"] Statement,
    With => [b"with"] Statement,
    Write => [b"write"] Statement,
    Any => [b"any"] Marker,
    As => [b"as"] Marker,
    ByRef => [b"byref"] Marker,
    ByVal => [b"byval"] Marker,
    Each => [b"each"] Marker,
    In => [b"in"] Marker,
    Shared => [b"shared"] Marker,
    Until => [b"until"] Marker,
    WithEvents => [b"withevents"] Marker,
    Optional => [b"optional"] Marker,
    ParamArray => [b"paramarray"] Marker,
    Preserve => [b"preserve"] Marker,
    Spc => [b"spc"] Marker,
    Tab => [b"tab"] Marker,
    Then => [b"then"] Marker,
    To => [b"to"] Marker,
    AddressOf => [b"addressof"] Operator,
    And => [b"and"] Operator,
    Eqv => [b"eqv"] Operator,
    Imp => [b"imp"] Operator,
    Is => [b"is"] Operator,
    Like => [b"like"] Operator,
    New => [b"new"] Operator,
    Mod => [b"mod"] Operator,
    Not => [b"not"] Operator,
    Or => [b"or"] Operator,
    TypeOf => [b"typeof"] Operator,
    Xor => [b"xor"] Operator,
    Abs => [b"abs"] ReservedName,
    CBool => [b"cbool"] ReservedName,
    CByte => [b"cbyte"] ReservedName,
    CCur => [b"ccur"] ReservedName,
    CDate => [b"cdate"] ReservedName,
    CDbl => [b"cdbl"] ReservedName,
    CDec => [b"cdec"] ReservedName,
    CInt => [b"cint"] ReservedName,
    CLng => [b"clng"] ReservedName,
    CLngLng => [b"clnglng"] ReservedName,
    CLngPtr => [b"clngptr"] ReservedName,
    CSng => [b"csng"] ReservedName,
    CStr => [b"cstr"] ReservedName,
    CVar => [b"cvar"] ReservedName,
    CVErr => [b"cverr"] ReservedName,
    Date => [b"date"] ReservedName,
    Debug => [b"debug"] ReservedName,
    DoEvents => [b"doevents"] ReservedName,
    Fix => [b"fix"] ReservedName,
    Int => [b"int"] ReservedName,
    Len => [b"len"] ReservedName,
    LenB => [b"lenb"] ReservedName,
    Me => [b"me"] ReservedName,
    PSet => [b"pset"] ReservedName,
    Scale => [b"scale"] ReservedName,
    Sgn => [b"sgn"] ReservedName,
    String => [b"string"] ReservedName,
    Array => [b"array"] SpecialForm,
    Circle => [b"circle"] SpecialForm,
    InputB => [b"inputb"] SpecialForm,
    LBound => [b"lbound"] SpecialForm,
    UBound => [b"ubound"] SpecialForm,
    Boolean => [b"boolean"] ReservedType,
    Byte => [b"byte"] ReservedType,
    Currency => [b"currency"] ReservedType,
    Double => [b"double"] ReservedType,
    Integer => [b"integer"] ReservedType,
    Long => [b"long"] ReservedType,
    LongLong => [b"longlong"] ReservedType,
    LongPtr => [b"longptr"] ReservedType,
    Single => [b"single"] ReservedType,
    Variant => [b"variant"] ReservedType,
    True => [b"true"] Literal,
    False => [b"false"] Literal,
    Nothing => [b"nothing"] Literal,
    Empty => [b"empty"] Literal,
    Null => [b"null"] Literal,
    Attribute => [b"attribute"] ImplementationUse,
    LineInput => [b"lineinput"] ImplementationUse,
    CDecl => [b"cdecl"] FutureReserved,
    Decimal => [b"decimal"] FutureReserved,
    DefDec => [b"defdec"] FutureReserved,
    PtrSafe => [b"ptrsafe"] Contextual,
    VbAttribute => [b"vb_base", b"vb_control", b"vb_creatable", b"vb_customizable", b"vb_description", b"vb_exposed", b"vb_ext_key", b"vb_globalnamespace", b"vb_helpid", b"vb_invoke_func", b"vb_invoke_property", b"vb_invoke_propertyput", b"vb_invoke_propertyputref", b"vb_memberflags", b"vb_name", b"vb_predeclaredid", b"vb_procdata", b"vb_templatederived", b"vb_usermemid", b"vb_vardescription", b"vb_varhelpid", b"vb_varmemberflags", b"vb_varprocdata", b"vb_varusermemid"] ImplementationUse,
}

impl Keyword {
    /// The language feature this word belongs to, if its availability depends
    /// on the language generation.
    pub const fn required_feature(self) -> Option<Feature> {
        match self {
            Keyword::LongPtr | Keyword::DefLngPtr | Keyword::CLngPtr => Some(Feature::LongPtr),
            Keyword::PtrSafe => Some(Feature::PtrSafe),
            Keyword::LongLong | Keyword::DefLngLng | Keyword::CLngLng => Some(Feature::LongLong),
            Keyword::Implements => Some(Feature::Implements),
            Keyword::Event | Keyword::RaiseEvent | Keyword::WithEvents => Some(Feature::Events),
            Keyword::AddressOf => Some(Feature::AddressOf),
            _ => None,
        }
    }

    /// Whether the word is reserved in `generation`. In generations that lack
    /// the feature the word is an ordinary identifier.
    pub const fn is_available_in(self, generation: LanguageGeneration) -> bool {
        match self.required_feature() {
            Some(f) => generation.supports(f),
            None => true,
        }
    }
}

/// Longest keyword spelling in bytes (`vb_invoke_propertyputref`).
const MAX_KEYWORD_LEN: usize = 24;

/// Looks up `text` (any ASCII case) as a keyword of `generation`.
pub fn lookup_keyword(text: &[u8], generation: LanguageGeneration) -> Option<Keyword> {
    if text.len() < 2 || text.len() > MAX_KEYWORD_LEN {
        return None;
    }
    let mut buf = [0u8; MAX_KEYWORD_LEN];
    let lower = &mut buf[..text.len()];
    for (d, s) in lower.iter_mut().zip(text) {
        *d = s.to_ascii_lowercase();
    }
    Keyword::from_lower(lower).filter(|k| k.is_available_in(generation))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_insensitive() {
        let g = LanguageGeneration::Vba7_1;
        assert_eq!(lookup_keyword(b"DIM", g), Some(Keyword::Dim));
        assert_eq!(lookup_keyword(b"eNdIf", g), Some(Keyword::EndIf));
        assert_eq!(lookup_keyword(b"vb_name", g), Some(Keyword::VbAttribute));
        assert_eq!(
            lookup_keyword(b"VB_Invoke_PropertyPutRef", g),
            Some(Keyword::VbAttribute)
        );
        assert_eq!(lookup_keyword(b"Dimm", g), None);
        assert_eq!(lookup_keyword(b"", g), None);
        assert_eq!(lookup_keyword(&[b'a'; 40], g), None);
    }

    #[test]
    fn generation_gating() {
        for (w, k) in [
            (&b"LongPtr"[..], Keyword::LongPtr),
            (b"LongLong", Keyword::LongLong),
            (b"PtrSafe", Keyword::PtrSafe),
            (b"CLngLng", Keyword::CLngLng),
            (b"DefLngPtr", Keyword::DefLngPtr),
        ] {
            assert_eq!(lookup_keyword(w, LanguageGeneration::Vba7_0), Some(k));
            assert_eq!(lookup_keyword(w, LanguageGeneration::Vba7_1), Some(k));
            assert_eq!(lookup_keyword(w, LanguageGeneration::Vba6), None);
            assert_eq!(lookup_keyword(w, LanguageGeneration::Vba5), None);
        }
        assert_eq!(
            lookup_keyword(b"Implements", LanguageGeneration::Access95),
            None
        );
        assert_eq!(
            lookup_keyword(b"Implements", LanguageGeneration::Vba5),
            Some(Keyword::Implements)
        );
    }

    #[test]
    fn spec_counts() {
        // Distinct words in the MS-VBAL lists plus PtrSafe and the VB_ group.
        assert_eq!(Keyword::ALL.len(), 149);
        assert_eq!(Keyword::True.class(), KeywordClass::Literal);
        assert_eq!(Keyword::And.class(), KeywordClass::Operator);
        assert_eq!(Keyword::Integer.class(), KeywordClass::ReservedType);
    }
}
