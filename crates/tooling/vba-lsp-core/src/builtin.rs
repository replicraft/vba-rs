//! Static knowledge about the language: keyword descriptions and the
//! presentation of standard-library function signatures.

use vba_lexer::{Keyword, KeywordClass};
use vba_stdlib::FunctionInfo;
use vba_value::VarType;

/// A one-line description of a keyword.
pub(crate) fn keyword_doc(word: &str, k: Keyword) -> String {
    let doc = match word.to_ascii_lowercase().as_str() {
        "dim" => "Declares variables and allocates storage: `Dim name [As Type]`.",
        "redim" => "Resizes a dynamic array: `ReDim [Preserve] name(bounds) [As Type]`.",
        "const" => "Declares a constant: `Const name [As Type] = expression`.",
        "static" => "Declares a variable that keeps its value between calls of the procedure.",
        "private" => "Limits a declaration to the module that contains it.",
        "public" => "Makes a declaration visible to all modules of the project.",
        "friend" => "Makes a class member visible inside the project but not outside it.",
        "global" => "Obsolete form of `Public` at module level.",
        "sub" => "Starts a procedure that does not return a value: `Sub name(params) ... End Sub`.",
        "function" => {
            "Starts a procedure that returns a value: `Function name(params) [As Type] ... End Function`."
        }
        "property" => {
            "Starts a property procedure: `Property Get`, `Property Let` or `Property Set`."
        }
        "get" => "Marks a property procedure that reads the property.",
        "let" => {
            "Assigns a value to a variable; optional in `Let x = 1`. Also marks a `Property Let` procedure."
        }
        "set" => {
            "Assigns an object reference: `Set x = obj`. Also marks a `Property Set` procedure."
        }
        "if" => {
            "Runs statements depending on a condition: `If cond Then ... [ElseIf ...] [Else ...] End If`."
        }
        "then" => "Introduces the statements run when an `If` or `ElseIf` condition is true.",
        "else" => {
            "Introduces the statements run when no earlier `If` or `ElseIf` condition was true."
        }
        "elseif" => "Tests another condition in an `If` block.",
        "select" => "Starts a multi-way branch: `Select Case expr ... End Select`.",
        "case" => "A branch of `Select Case`; `Case Else` runs when no other case matched.",
        "for" => {
            "Counted loop `For i = a To b [Step s] ... Next [i]` or `For Each x In collection`."
        }
        "each" => "Part of `For Each element In group`.",
        "to" => "Separates the bounds of a range, in `For`, `Case` and array bounds.",
        "step" => "Sets the increment of a `For` loop.",
        "next" => "Ends a `For` loop.",
        "do" => "Starts a `Do ... Loop` loop with an optional `While` or `Until` condition.",
        "loop" => "Ends a `Do` loop.",
        "while" => "Loop condition, or the older `While ... Wend` loop.",
        "wend" => "Ends a `While` loop.",
        "until" => "Loop condition: repeat until it becomes true.",
        "with" => "Runs statements against one object: `With obj ... End With`.",
        "exit" => {
            "Leaves a procedure or loop: `Exit Sub`, `Exit Function`, `Exit Property`, `Exit For`, `Exit Do`."
        }
        "end" => "Ends a block (`End If`, `End Sub`, ...); alone, stops the program.",
        "goto" => "Jumps to a label in the same procedure.",
        "gosub" => "Calls a subroutine at a label; `Return` comes back.",
        "return" => "Returns from a `GoSub`.",
        "on" => {
            "`On Error GoTo label | Resume Next | GoTo 0` sets error handling; `On expr GoTo` branches by value."
        }
        "resume" => "Continues after an error: `Resume`, `Resume Next` or `Resume label`.",
        "call" => "Calls a procedure: `Call name(args)`.",
        "option" => {
            "Module-level option: `Option Explicit`, `Option Base 0|1`, `Option Compare Text|Binary|Database`, `Option Private Module`."
        }
        "declare" => {
            "Declares an external procedure: `Declare [PtrSafe] Sub|Function name Lib \"dll\" ...`."
        }
        "ptrsafe" => "Marks a `Declare` as safe for 64-bit pointers (VBA 7).",
        "type" => "Declares a user-defined type: `Type name ... End Type`.",
        "enum" => "Declares an enumeration of named Long constants: `Enum name ... End Enum`.",
        "event" => "Declares an event a class can raise.",
        "raiseevent" => "Raises an event declared with `Event`.",
        "withevents" => "Declares an object variable whose events can be handled.",
        "implements" => "Declares that a class implements the members of an interface.",
        "as" => "Introduces a type: `x As Long`.",
        "new" => "Creates a new object instance: `New ClassName`.",
        "byval" => "Passes an argument by value.",
        "byref" => "Passes an argument by reference (the default).",
        "optional" => "Marks a parameter that may be omitted.",
        "paramarray" => "Collects any number of trailing arguments into a Variant array.",
        "addressof" => "Gets the address of a procedure, for callbacks.",
        "typeof" => "`TypeOf obj Is Type` tests the type of an object.",
        "not" => "Logical or bitwise negation.",
        "and" => "Logical or bitwise conjunction.",
        "or" => "Logical or bitwise disjunction.",
        "xor" => "Logical or bitwise exclusive or.",
        "eqv" => "Logical or bitwise equivalence.",
        "imp" => "Logical or bitwise implication.",
        "mod" => "Remainder of integer division.",
        "like" => "Pattern comparison of strings.",
        "is" => "Compares object references; also used in `Case Is` and `TypeOf ... Is`.",
        "true" => "The Boolean value True (-1).",
        "false" => "The Boolean value False (0).",
        "nothing" => "Clears an object reference.",
        "empty" => "The value of an uninitialised Variant.",
        "null" => "A Variant that contains no valid data.",
        "me" => "The object whose code is running (class modules and forms).",
        "erase" => "Reinitialises the elements of arrays.",
        "preserve" => "Keeps the data of an array in `ReDim`.",
        "open" => "Opens a file: `Open path For mode As #n`.",
        "close" => "Closes open files.",
        "print" => "Writes formatted text to a file: `Print #n, items`.",
        "write" => "Writes delimited data to a file: `Write #n, items`.",
        "input" => "Reads data from a file: `Input #n, variables`.",
        "stop" => "Suspends execution, like a breakpoint.",
        "attribute" => "Module or member attribute, written by the editor in exported files.",
        "debug" => "The `Immediate` window object: `Debug.Print`.",
        "defbool" | "defbyte" | "defcur" | "defdate" | "defdbl" | "defint" | "deflng"
        | "deflnglng" | "deflngptr" | "defobj" | "defsng" | "defstr" | "defvar" => {
            "Sets the default type for names starting with the given letters."
        }
        "boolean" => "Type: True or False (2 bytes).",
        "byte" => "Type: unsigned 8-bit integer, 0 to 255.",
        "currency" => "Type: fixed-point number with four decimals (8 bytes).",
        "double" => "Type: 64-bit floating point.",
        "integer" => "Type: 16-bit signed integer, -32,768 to 32,767.",
        "long" => "Type: 32-bit signed integer.",
        "longlong" => "Type: 64-bit signed integer (64-bit VBA 7 only).",
        "longptr" => "Type: pointer-sized integer, Long on 32-bit and LongLong on 64-bit (VBA 7).",
        "single" => "Type: 32-bit floating point.",
        "string" => "Type: text, variable length or fixed length with `* n`.",
        "variant" => "Type: can hold any value; the default type.",
        "date" => "Type: date and time (a Double). As a function, returns the current date.",
        _ => "",
    };
    if !doc.is_empty() {
        return doc.to_string();
    }
    let class = match k.class() {
        KeywordClass::Statement => "statement keyword",
        KeywordClass::Marker => "keyword used inside statements",
        KeywordClass::Operator => "operator",
        KeywordClass::ReservedName => "reserved name",
        KeywordClass::SpecialForm => "built-in special form",
        KeywordClass::ReservedType => "built-in type",
        KeywordClass::Literal => "literal",
        KeywordClass::ImplementationUse => "reserved for implementation use",
        KeywordClass::FutureReserved => "reserved for future use",
        KeywordClass::Contextual => "contextual keyword",
    };
    format!("`{word}` is a VBA {class}.")
}

/// Display name of a type in a signature.
pub(crate) fn type_name(t: VarType) -> &'static str {
    match t {
        VarType::Empty => "Empty",
        VarType::Null => "Null",
        VarType::Integer => "Integer",
        VarType::Long => "Long",
        VarType::Single => "Single",
        VarType::Double => "Double",
        VarType::Currency => "Currency",
        VarType::Date => "Date",
        VarType::String => "String",
        VarType::Object => "Object",
        VarType::Error => "Error",
        VarType::Boolean => "Boolean",
        VarType::Variant => "Variant",
        VarType::DataObject => "Object",
        VarType::Decimal => "Decimal",
        VarType::Byte => "Byte",
        VarType::LongLong => "LongLong",
        VarType::UserDefinedType => "Type",
    }
}

/// A rendered stdlib signature: the label and the byte ranges of each
/// parameter in it (the label is ASCII, so bytes equal UTF-16 units).
pub(crate) struct StdSig {
    pub label: String,
    pub params: Vec<(u32, u32)>,
}

pub(crate) fn std_signature(f: &FunctionInfo) -> StdSig {
    let mut label = format!("{}(", f.name);
    let mut params = Vec::new();
    for (i, p) in f.params.iter().enumerate() {
        if i > 0 {
            label.push_str(", ");
        }
        let start = label.len() as u32;
        let mut text = String::new();
        if p.optional {
            text.push('[');
        }
        if p.param_array {
            text.push_str("ParamArray ");
        }
        text.push_str(p.name);
        text.push_str(" As ");
        text.push_str(type_name(p.ty));
        if let Some(d) = p.default {
            text.push_str(" = ");
            text.push_str(d);
        }
        if p.optional {
            text.push(']');
        }
        label.push_str(&text);
        params.push((start, label.len() as u32));
    }
    label.push(')');
    if f.returns != VarType::Empty {
        label.push_str(" As ");
        label.push_str(type_name(f.returns));
    }
    StdSig { label, params }
}

/// Markdown documentation for a stdlib function.
pub(crate) fn std_doc(f: &FunctionInfo) -> String {
    let mut s = format!("VBA library function (`VBA.{}.{}`).", f.module, f.name);
    if f.dollar {
        s.push_str(&format!(
            " `{}$` returns a plain String and fails on Null.",
            f.name
        ));
    }
    let min = f.min_args();
    match f.max_args() {
        Some(max) if max == min => s.push_str(&format!(" Takes {min} argument(s).")),
        Some(max) => s.push_str(&format!(" Takes {min} to {max} arguments.")),
        None => s.push_str(&format!(" Takes at least {min} argument(s).")),
    }
    s
}
