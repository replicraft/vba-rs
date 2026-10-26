//! The seam between the interpreter and the services it does not own: file
//! statements and the file system, `Shell`, `Command`.
//!
//! The VM evaluates operands, converts them to plain values and hands them
//! over as an [`IoRequest`]; the embedder (normally `vba-runtime`) decides
//! what a file is, checks the host policy and returns an [`IoReply`]. The VM
//! never touches the operating system.

use vba_ast::{OpenAccess, OpenLock, OpenMode};
use vba_automation::ErrInfo;
use vba_value::{VarType, Variant};

/// One item of a `Print`, `Write` or `Debug.Print` list.
#[derive(Debug, Clone, PartialEq)]
pub enum PrintItem {
    /// A value.
    Value(Variant),
    /// `;`
    Semicolon,
    /// `,` (next print zone)
    Comma,
    /// `Tab(n)`; `None` is a bare `Tab`.
    Tab(Option<i32>),
    /// `Spc(n)`.
    Spc(i32),
}

/// The layout of a record for `Get` and `Put`, derived from the static type.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A scalar of the given subtype (`Variant` for a `Variant`; `String` for
    /// a variable-length string).
    Scalar(VarType),
    /// A variable-length `String`; `current_len` is the length of the value
    /// currently in the target, which `Get` in binary mode uses as the byte
    /// count.
    VarString {
        /// Current length in characters.
        current_len: usize,
    },
    /// `String * n`.
    FixedString(u32),
    /// A user-defined type with its field shapes in order.
    Udt(Vec<Shape>),
    /// A fixed-size array (elements in storage order).
    Array {
        /// Element shape.
        elem: Box<Shape>,
        /// Number of elements.
        count: usize,
    },
}

/// A record value matching a [`Shape`].
#[derive(Debug, Clone, PartialEq)]
pub enum RecordValue {
    /// A scalar.
    Scalar(Variant),
    /// The fields of a user-defined type, or the elements of an array.
    Fields(Vec<RecordValue>),
}

/// A file statement with evaluated operands.
#[derive(Debug, Clone, PartialEq)]
pub enum IoRequest {
    /// `Open path For mode [Access a] [Lock l] As #file [Len = n]`.
    Open {
        /// Path as written.
        path: String,
        /// Mode.
        mode: OpenMode,
        /// Access clause.
        access: Option<OpenAccess>,
        /// Lock clause.
        lock: Option<OpenLock>,
        /// File number.
        file: i32,
        /// Record length.
        record_len: Option<i32>,
    },
    /// `Close` (empty closes every file).
    Close(Vec<i32>),
    /// `Print #file, items`.
    Print {
        /// File number.
        file: i32,
        /// Items.
        items: Vec<PrintItem>,
    },
    /// `Write #file, items`.
    Write {
        /// File number.
        file: i32,
        /// Items.
        items: Vec<PrintItem>,
    },
    /// `Input #file, targets`; the kinds are the declared target types
    /// (`Variant` parses the token by its content).
    Input {
        /// File number.
        file: i32,
        /// Target subtypes.
        targets: Vec<VarType>,
    },
    /// `Line Input #file, target`.
    LineInput {
        /// File number.
        file: i32,
    },
    /// `Get #file, [record], target`.
    Get {
        /// File number.
        file: i32,
        /// Record number (random) or byte position (binary), 1-based.
        record: Option<i64>,
        /// Layout of the target.
        shape: Shape,
    },
    /// `Put #file, [record], value`.
    Put {
        /// File number.
        file: i32,
        /// Record number or byte position, 1-based.
        record: Option<i64>,
        /// Layout of the value.
        shape: Shape,
        /// The value.
        value: RecordValue,
    },
    /// `Seek #file, position`.
    Seek {
        /// File number.
        file: i32,
        /// New position.
        position: i64,
    },
    /// `Lock #file [, from [To to]]`.
    Lock {
        /// File number.
        file: i32,
        /// Range start.
        from: Option<i64>,
        /// Range end.
        to: Option<i64>,
    },
    /// `Unlock #file [, from [To to]]`.
    Unlock {
        /// File number.
        file: i32,
        /// Range start.
        from: Option<i64>,
        /// Range end.
        to: Option<i64>,
    },
    /// `Width #file, n`.
    Width {
        /// File number.
        file: i32,
        /// Width.
        width: i32,
    },
    /// `Name old As new`.
    Name {
        /// Old path.
        old: String,
        /// New path.
        new: String,
    },
}

/// The result of a file statement.
#[derive(Debug, Clone, PartialEq)]
pub enum IoReply {
    /// Nothing to return.
    Done,
    /// The values read by `Input #`, one per target.
    Values(Vec<Variant>),
    /// The line read by `Line Input #`.
    Line(vba_value::VbaString),
    /// The record read by `Get`.
    Record(RecordValue),
}

/// A failure reported by [`Services`].
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceError {
    /// A VBA error (file not found, bad file number, permission denied).
    Vba(ErrInfo),
    /// A feature that is not implemented; surfaced as an untrappable,
    /// explicit error.
    Unsupported(String),
    /// A resource limit was exceeded (memory); untrappable.
    Limit(crate::LimitKind),
}

impl From<ErrInfo> for ServiceError {
    fn from(e: ErrInfo) -> Self {
        ServiceError::Vba(e)
    }
}

/// Services outside the interpreter.
pub trait Services {
    /// Runs a file statement.
    fn io(&mut self, req: IoRequest) -> Result<IoReply, ServiceError>;

    /// Calls a run-time function or sub the VM does not implement itself:
    /// `Shell`, `Command`, `FreeFile`, `EOF`, `LOF`, `Loc`, `Seek`, `Kill`,
    /// `MkDir`, `RmDir`, `ChDir`, `ChDrive`, `CurDir`, `Dir`, `FileCopy`,
    /// `FileLen`, `FileDateTime`, `GetAttr`, `SetAttr`. Names are
    /// case-insensitive; omitted arguments are `None`.
    fn call(&mut self, name: &str, args: &[Option<Variant>]) -> Result<Variant, ServiceError>;

    /// Closes everything (`End`, `Reset`).
    fn reset(&mut self) {}
}

/// Services for a program that must not touch files or processes: every
/// request is an explicit "unsupported" error.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoServices;

impl Services for NoServices {
    fn io(&mut self, _: IoRequest) -> Result<IoReply, ServiceError> {
        Err(ServiceError::Unsupported(
            "file statements need a Services implementation".into(),
        ))
    }

    fn call(&mut self, name: &str, _: &[Option<Variant>]) -> Result<Variant, ServiceError> {
        Err(ServiceError::Unsupported(format!(
            "`{name}` needs a Services implementation"
        )))
    }
}
