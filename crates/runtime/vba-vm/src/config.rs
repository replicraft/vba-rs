//! Configuration and the `Debug.Print` sink.

use std::cell::RefCell;
use std::rc::Rc;

/// Receives the text written by `Debug.Print`.
pub trait DebugSink {
    /// Appends text. A `Debug.Print` statement ends with `\n` unless its
    /// list ends with `;` or `,`.
    fn write(&mut self, text: &str);
}

/// A sink that keeps everything in a shared buffer. The buffer is charged
/// to the memory budget of the run that writes to it
/// (`max_memory_bytes`) and credited when the last clone is dropped.
#[derive(Debug, Clone, Default)]
pub struct BufferSink {
    buffer: Rc<RefCell<String>>,
    charge: Rc<RefCell<vba_core::Charge>>,
}

impl BufferSink {
    /// An empty buffer.
    pub fn new() -> BufferSink {
        BufferSink::default()
    }

    /// A handle to the shared buffer; it stays valid after the sink moved
    /// into a VM.
    pub fn buffer(&self) -> Rc<RefCell<String>> {
        self.buffer.clone()
    }

    /// The text written so far.
    pub fn contents(&self) -> String {
        self.buffer.borrow().clone()
    }
}

impl DebugSink for BufferSink {
    fn write(&mut self, text: &str) {
        let mut b = self.buffer.borrow_mut();
        b.push_str(text);
        self.charge.borrow_mut().set(b.capacity() as u64);
    }
}

/// A sink that discards everything.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullSink;

impl DebugSink for NullSink {
    fn write(&mut self, _: &str) {}
}

/// Settings of a VM instance.
#[derive(Debug, Clone)]
pub struct VmConfig {
    /// The project name, the default `Err.Source`.
    pub project_name: String,
    /// The longest string a single operation may produce, in UTF-16 units;
    /// longer results are an untrappable limit fault.
    pub max_string_units: usize,
    /// The largest array, in elements; `ReDim` beyond it is error 7.
    pub max_array_elements: u64,
    /// The deepest `GoSub` nesting of one frame (error 28 beyond it).
    pub max_gosub_depth: usize,
    /// How deeply the interpreter may nest itself on the native stack. Calls
    /// between procedures never nest it; `Class_Initialize`,
    /// `Class_Terminate` bodies reached from inside another one, event
    /// handlers and default-member coercions do. Beyond the limit the run
    /// ends with a call-depth limit error.
    pub max_native_depth: u32,
    /// How much native stack, in bytes, nested runs may use between the
    /// outermost run and the innermost one. The count above does not bound
    /// the stack by itself: a nested run reached through host code, a
    /// default-member coercion or a deep chain of native frames costs a
    /// different amount each time. Both limits apply and the first one hit
    /// ends the run with a call-depth limit error. The default (1 MiB) fits a
    /// thread of the standard library's default 2 MiB; a debug build spends
    /// about 70 KiB on a level. Lower it when the interpreter runs on a
    /// thread with a smaller stack.
    pub max_native_stack_bytes: usize,
    /// How many runs may nest through host calls: a host object that
    /// declares itself re-entrant calls back into the session (raising an
    /// event, creating a form) while the VM waits in that host call. Beyond
    /// this depth the nested operation fails, see
    /// [`ReentryError::TooDeep`](crate::ReentryError::TooDeep).
    pub max_reentrancy_depth: u32,
    /// Mixed into the opaque identity tokens `ObjPtr`, `StrPtr` and `VarPtr`
    /// return. The tokens are not addresses and cannot be dereferenced; the
    /// salt only keeps their values from being predictable across sessions
    /// that choose different salts. The default is fixed, so that a run is
    /// reproducible; an embedding application that wants unpredictable
    /// values sets its own.
    pub identity_salt: u64,
}

impl Default for VmConfig {
    fn default() -> VmConfig {
        VmConfig {
            project_name: "VBAProject".to_owned(),
            max_string_units: 1 << 25,
            max_array_elements: 1 << 24,
            max_gosub_depth: 1 << 12,
            max_native_depth: 48,
            max_native_stack_bytes: 1 << 20,
            max_reentrancy_depth: 4,
            identity_salt: 0x9E37_79B9_7F4A_7C15,
        }
    }
}
