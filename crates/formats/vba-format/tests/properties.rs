//! Property tests for the compressor, the decompressor and project text.

mod common;

use common::*;
use proptest::prelude::*;
use vba_format::{Limits, WriteOptions, compress, decompress, read_project, write_project};
use vba_project::{Module, ModuleKind, VbaProject};

const LIMIT: usize = 1 << 24;

/// Compressible data: runs and repeated phrases mixed with random bytes.
fn structured() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(
        prop_oneof![
            (any::<u8>(), 1usize..300).prop_map(|(b, n)| vec![b; n]),
            prop::collection::vec(any::<u8>(), 1..40),
            Just(b"Public Sub Example()\r\n    Dim x As Long\r\nEnd Sub\r\n".to_vec()),
        ],
        0..80,
    )
    .prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Decompressing a compressed buffer yields the input, plus zero padding
    /// only when the final chunk had to be stored raw (§2.4.1.3.10).
    #[test]
    fn compress_round_trips_random_bytes(data in prop::collection::vec(any::<u8>(), 0..9000)) {
        let out = decompress(&compress(&data), LIMIT).unwrap();
        prop_assert!(out.len() >= data.len());
        prop_assert_eq!(&out[..data.len()], &data[..]);
        prop_assert!(out[data.len()..].iter().all(|&b| b == 0));
        prop_assert!(out.len() - data.len() < 4096);
    }

    #[test]
    fn compress_round_trips_structured_bytes(data in structured()) {
        let packed = compress(&data);
        let out = decompress(&packed, LIMIT).unwrap();
        prop_assert_eq!(&out[..data.len()], &data[..]);
        prop_assert!(out[data.len()..].iter().all(|&b| b == 0));
        // Deterministic.
        prop_assert_eq!(packed, compress(&data));
    }

    /// Input of fewer than 3640 bytes is never padded, so round trips are exact.
    #[test]
    fn short_inputs_round_trip_exactly(data in prop::collection::vec(any::<u8>(), 0..3600)) {
        prop_assert_eq!(decompress(&compress(&data), LIMIT).unwrap(), data);
    }

    #[test]
    fn decompress_never_panics_on_arbitrary_bytes(mut data in prop::collection::vec(any::<u8>(), 0..600)) {
        if let Some(first) = data.first_mut() {
            *first = 1;
        }
        if let Ok(out) = decompress(&data, 1 << 16) {
            prop_assert!(out.len() <= 1 << 16);
        }
    }

    #[test]
    fn decompress_never_panics_on_mutated_streams(
        data in structured(),
        edits in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..6),
    ) {
        let mut packed = compress(&data);
        for (idx, byte) in edits {
            let i = idx.index(packed.len());
            packed[i] = byte;
        }
        let _ = decompress(&packed, 1 << 20);
    }

    /// Any text the code page can hold survives a write/read cycle of a new
    /// project, including line-ending variants and non-ASCII characters.
    #[test]
    fn module_source_round_trips_through_a_project(
        lines in prop::collection::vec("[ -~äöüßé€]{0,60}", 0..40),
        crlf in any::<bool>(),
    ) {
        let eol = if crlf { "\r\n" } else { "\n" };
        let source: String = lines.iter().map(|l| format!("{l}{eol}")).collect();
        let mut p = VbaProject::new("P");
        p.add_module(Module::new("M1", ModuleKind::Standard, source.clone())).unwrap();
        let back = read_project(&write_project(&p, &WriteOptions::default()).unwrap(), &Limits::default()).unwrap();
        prop_assert_eq!(&back.module("M1").unwrap().source, &source);
    }

    /// Replacing source in a project read from a file keeps all other modules
    /// and unknown streams intact.
    #[test]
    fn editing_keeps_other_modules_and_unknown_streams(
        lines in prop::collection::vec("[ -~]{0,50}", 0..30),
        target in 0usize..5,
    ) {
        let bin = sample_project_bin();
        let mut p = read_project(&bin, &Limits::default()).unwrap();
        let name = p.modules[target].name.clone();
        let source: String = lines.iter().map(|l| format!("{l}\r\n")).collect();
        p.set_module_source(&name, source.clone()).unwrap();
        let back = read_project(&write_project(&p, &WriteOptions::default()).unwrap(), &Limits::default()).unwrap();
        for (i, m) in back.modules.iter().enumerate() {
            let orig = &read_project(&bin, &Limits::default()).unwrap().modules[i];
            if i == target {
                prop_assert_eq!(&m.source, &source);
            } else {
                prop_assert_eq!(&m.source, &orig.source);
            }
        }
        let orig_streams = dump_cfb(&bin).0;
        let new_streams = dump_cfb(&write_project(&p, &WriteOptions::default()).unwrap()).0;
        for (path, data) in orig_streams {
            if path.starts_with("/UserForm1/") || path.starts_with("/Unknown/")
                || path == "/XTRA" || path == "/PROJECT" || path == "/PROJECTwm"
            {
                prop_assert!(new_streams.iter().any(|(p, d)| *p == path && *d == data), "{}", path);
            }
        }
    }
}
