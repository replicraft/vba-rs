//! Configuration and the `Debug.Print` sink.

use std::cell::RefCell;
use std::rc::Rc;

/// Receives the text written by `Debug.Print`.
pub trait DebugSink {
    /// Appends text. A `Debug.Print` statement ends with `\n` unless its
    /// list ends with `;` or `,`.
    fn write(&mut self, text: &str);
}

/// A sink that keeps everything in a shared buffer.
#[derive(Debug, Clone, Default)]
pub struct BufferSink {
    buffer: Rc<RefCell<String>>,
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
        self.buffer.borrow_mut().push_str(text);
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
}

impl Default for VmConfig {
    fn default() -> VmConfig {
        VmConfig {
            project_name: "VBAProject".to_owned(),
            max_string_units: 1 << 25,
            max_array_elements: 1 << 24,
            max_gosub_depth: 1 << 12,
            max_native_depth: 48,
        }
    }
}
