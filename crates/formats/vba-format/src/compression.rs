//! MS-OVBA §2.4.1 compression container.
//!
//! A container is a `0x01` signature byte followed by chunks, each covering up
//! to 4096 decompressed bytes.

/// Decompressed bytes per chunk.
const CHUNK_SIZE: usize = 4096;
const SIGNATURE_BYTE: u8 = 0x01;
/// Bits 12..=14 of a chunk header must hold this value.
const CHUNK_SIGNATURE: u16 = 0b011;

/// Reasons a compressed container is rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecompressError {
    /// The input is empty or does not start with `0x01`.
    #[error("missing compressed container signature byte")]
    BadSignature,
    /// A chunk header is incomplete or its signature bits are not `0b011`.
    #[error("invalid chunk header at offset {offset}")]
    BadChunkHeader {
        /// Offset of the header in the compressed input.
        offset: usize,
    },
    /// The chunk declares more bytes than the input holds.
    #[error("chunk at offset {offset} is truncated")]
    TruncatedChunk {
        /// Offset of the chunk header in the compressed input.
        offset: usize,
    },
    /// A copy token ended mid-token or a chunk produced more than 4096 bytes.
    #[error("malformed token data in chunk at offset {offset}")]
    MalformedTokens {
        /// Offset of the chunk header in the compressed input.
        offset: usize,
    },
    /// A copy token refers before the start of its chunk.
    #[error("copy token offset out of range in chunk at offset {offset}")]
    BadCopyOffset {
        /// Offset of the chunk header in the compressed input.
        offset: usize,
    },
    /// Output would exceed the caller-supplied limit.
    #[error("decompressed size exceeds limit of {limit} bytes")]
    LimitExceeded {
        /// The limit that was exceeded.
        limit: usize,
    },
}

/// Decompresses an MS-OVBA compressed container.
///
/// `limit` caps the output size so hostile input cannot force huge allocations.
pub fn decompress(input: &[u8], limit: usize) -> Result<Vec<u8>, DecompressError> {
    if input.first() != Some(&SIGNATURE_BYTE) {
        return Err(DecompressError::BadSignature);
    }
    let mut out = Vec::new();
    let mut pos = 1;
    while pos < input.len() {
        let header_at = pos;
        let header = input
            .get(pos..pos + 2)
            .ok_or(DecompressError::BadChunkHeader { offset: header_at })?;
        let header = u16::from_le_bytes([header[0], header[1]]);
        if (header >> 12) & 0b111 != CHUNK_SIGNATURE {
            return Err(DecompressError::BadChunkHeader { offset: header_at });
        }
        let compressed = header & 0x8000 != 0;
        // The size field stores the chunk length (header included) minus 3.
        let chunk_len = usize::from(header & 0x0FFF) + 3;
        let data = input
            .get(pos + 2..pos + chunk_len)
            .ok_or(DecompressError::TruncatedChunk { offset: header_at })?;
        pos += chunk_len;

        if !compressed {
            if data.len() != CHUNK_SIZE {
                return Err(DecompressError::MalformedTokens { offset: header_at });
            }
            check_limit(out.len() + CHUNK_SIZE, limit)?;
            out.extend_from_slice(data);
        } else {
            decompress_chunk(data, header_at, &mut out, limit)?;
        }
    }
    Ok(out)
}

fn check_limit(len: usize, limit: usize) -> Result<(), DecompressError> {
    if len > limit {
        Err(DecompressError::LimitExceeded { limit })
    } else {
        Ok(())
    }
}

fn decompress_chunk(
    data: &[u8],
    header_at: usize,
    out: &mut Vec<u8>,
    limit: usize,
) -> Result<(), DecompressError> {
    let malformed = DecompressError::MalformedTokens { offset: header_at };
    let chunk_start = out.len();
    let mut i = 0;
    while i < data.len() {
        let flags = data[i];
        i += 1;
        for bit in 0..8 {
            if i >= data.len() {
                break;
            }
            let produced = out.len() - chunk_start;
            if flags >> bit & 1 == 0 {
                if produced >= CHUNK_SIZE {
                    return Err(malformed);
                }
                check_limit(out.len() + 1, limit)?;
                out.push(data[i]);
                i += 1;
            } else {
                let token = data.get(i..i + 2).ok_or_else(|| malformed.clone())?;
                i += 2;
                let token = u16::from_le_bytes([token[0], token[1]]);
                if produced == 0 {
                    return Err(DecompressError::BadCopyOffset { offset: header_at });
                }
                // The offset/length split depends on how much of the chunk
                // has been produced so far: ceil(log2(produced)), minimum 4.
                let bit_count = (usize::BITS - (produced - 1).leading_zeros()).max(4);
                let length_mask = 0xFFFFu16 >> bit_count;
                let length = usize::from(token & length_mask) + 3;
                let offset = usize::from((token & !length_mask) >> (16 - bit_count)) + 1;
                if offset > produced {
                    return Err(DecompressError::BadCopyOffset { offset: header_at });
                }
                if produced + length > CHUNK_SIZE {
                    return Err(malformed);
                }
                check_limit(out.len() + length, limit)?;
                // Copies may overlap their own output, so go byte by byte.
                let from = out.len() - offset;
                for k in 0..length {
                    out.push(out[from + k]);
                }
            }
        }
    }
    Ok(())
}

/// Compresses `input` into an MS-OVBA compressed container (§2.4.1.3.6).
///
/// The output is deterministic and follows the specification's pseudocode,
/// including its match selection rules (nearest candidate wins ties), so equal
/// input always yields equal bytes. A chunk that cannot be encoded in 4096
/// bytes or fewer is stored raw (§2.4.1.3.10).
///
/// A final chunk that is shorter than 4096 bytes *and* stored raw is padded
/// with zero bytes by the specification, so decompressing it yields trailing
/// zeros. This only happens for incompressible final chunks longer than about
/// 3640 bytes.
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() / 2 + 8);
    out.push(SIGNATURE_BYTE);
    for chunk in input.chunks(CHUNK_SIZE) {
        compress_chunk(chunk, &mut out);
    }
    out
}

/// Largest value of the chunk's `CompressedCurrent` (relative to the chunk
/// start): two header bytes plus 4096 data bytes.
const CHUNK_END: usize = CHUNK_SIZE + 2;

/// §2.4.1.3.7 Compressing a DecompressedChunk.
fn compress_chunk(chunk: &[u8], out: &mut Vec<u8>) {
    let mut data: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
    let mut current = 0; // DecompressedCurrent, relative to the chunk
    // CompressedCurrent is `2 + data.len()` unless the chunk was marked full.
    let mut full = false;
    while current < chunk.len() && !full && 2 + data.len() < CHUNK_END {
        compress_token_sequence(chunk, &mut current, &mut data, &mut full);
    }
    if current < chunk.len() {
        // Raw chunk: flag 0, size field 4095, zero padded (§2.4.1.3.10).
        out.extend_from_slice(&(0x3000u16 | (CHUNK_SIZE as u16 - 1)).to_le_bytes());
        out.extend_from_slice(chunk);
        out.resize(out.len() + CHUNK_SIZE - chunk.len(), 0);
    } else {
        let size = 2 + data.len();
        let header = 0xB000u16 | (size as u16 - 3);
        out.extend_from_slice(&header.to_le_bytes());
        out.extend_from_slice(&data);
    }
}

/// §2.4.1.3.8 Compressing a TokenSequence.
fn compress_token_sequence(chunk: &[u8], current: &mut usize, data: &mut Vec<u8>, full: &mut bool) {
    let flag_index = data.len();
    data.push(0);
    let mut flags = 0u8;
    for index in 0..8 {
        if *current >= chunk.len() || *full || 2 + data.len() >= CHUNK_END {
            break;
        }
        // §2.4.1.3.9 Compressing a Token.
        match find_match(chunk, *current) {
            Some((offset, length)) => {
                if 2 + data.len() + 1 < CHUNK_END {
                    let bit_count = copy_token_bits(*current);
                    let token = (((offset - 1) as u16) << (16 - bit_count)) | (length as u16 - 3);
                    data.extend_from_slice(&token.to_le_bytes());
                    flags |= 1 << index;
                    *current += length;
                } else {
                    *full = true;
                }
            }
            None => {
                if 2 + data.len() < CHUNK_END {
                    data.push(chunk[*current]);
                    *current += 1;
                } else {
                    *full = true;
                }
            }
        }
    }
    data[flag_index] = flags;
}

/// Number of offset bits in a copy token when `produced` bytes of the chunk
/// precede it (§2.4.1.3.19.1).
fn copy_token_bits(produced: usize) -> u32 {
    if produced <= 1 {
        4
    } else {
        (usize::BITS - (produced - 1).leading_zeros()).max(4)
    }
}

/// §2.4.1.3.19.4 Matching. Returns `(offset, length)` of the best copy
/// sequence, or `None` when no sequence of at least three bytes exists.
fn find_match(chunk: &[u8], current: usize) -> Option<(usize, usize)> {
    let mut best_len = 0;
    let mut best_candidate = 0;
    for candidate in (0..current).rev() {
        let len = chunk[candidate..]
            .iter()
            .zip(&chunk[current..])
            .take_while(|(a, b)| a == b)
            .count();
        // A later (further) candidate must be strictly longer to win.
        if len > best_len {
            best_len = len;
            best_candidate = candidate;
        }
    }
    if best_len >= 3 {
        let max_len = (0xFFFFusize >> copy_token_bits(current)) + 3;
        Some((current - best_candidate, best_len.min(max_len)))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: usize = 1 << 20;

    /// MS-OVBA §3.2.1 No Compression Example.
    const EX1_PLAIN: &[u8] = b"abcdefghijklmnopqrstuv.";
    const EX1_PACKED: &[u8] = &[
        0x01, 0x19, 0xB0, 0x00, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x00, 0x69, 0x6A,
        0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70, 0x00, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x2E,
    ];

    /// MS-OVBA §3.2.2 Normal Compression Example.
    const EX2_PLAIN: &[u8] = b"#aaabcdefaaaaghijaaaaaklaaamnopqaaaaaaaaaaaarstuvwxyzaaa";
    const EX2_PACKED: &[u8] = &[
        0x01, 0x2F, 0xB0, 0x00, 0x23, 0x61, 0x61, 0x61, 0x62, 0x63, 0x64, 0x65, 0x82, 0x66, 0x00,
        0x70, 0x61, 0x67, 0x68, 0x69, 0x6A, 0x01, 0x38, 0x08, 0x61, 0x6B, 0x6C, 0x00, 0x20, 0x6D,
        0x6E, 0x6F, 0x70, 0x06, 0x71, 0x02, 0x70, 0x04, 0x00, 0x72, 0x73, 0x74, 0x75, 0x76, 0x10,
        0x77, 0x78, 0x79, 0x7A, 0x00, 0x2C,
    ];

    /// MS-OVBA §3.2.3 Maximum Compression Example (73 times 'a').
    const EX3_PACKED: &[u8] = &[0x01, 0x03, 0xB0, 0x02, 0x61, 0x45, 0x00];

    fn ex3_plain() -> Vec<u8> {
        vec![b'a'; 73]
    }

    #[test]
    fn spec_examples_decompress() {
        assert_eq!(decompress(EX1_PACKED, LIMIT).unwrap(), EX1_PLAIN);
        assert_eq!(decompress(EX2_PACKED, LIMIT).unwrap(), EX2_PLAIN);
        assert_eq!(decompress(EX3_PACKED, LIMIT).unwrap(), ex3_plain());
    }

    #[test]
    fn spec_examples_compress_to_the_documented_bytes() {
        assert_eq!(compress(EX1_PLAIN), EX1_PACKED);
        assert_eq!(compress(EX2_PLAIN), EX2_PACKED);
        assert_eq!(compress(&ex3_plain()), EX3_PACKED);
    }

    #[test]
    fn empty_input_is_signature_only() {
        assert_eq!(compress(b""), [0x01]);
    }

    #[test]
    fn full_chunk_of_one_byte_is_tiny() {
        let packed = compress(&[b'a'; 4096]);
        assert!(packed.len() < 16, "{} bytes", packed.len());
        assert_eq!(decompress(&packed, LIMIT).unwrap(), vec![b'a'; 4096]);
    }

    /// Deterministic pseudo-random bytes (xorshift), incompressible in practice.
    fn noise(len: usize, mut seed: u64) -> Vec<u8> {
        (0..len)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (seed >> 24) as u8
            })
            .collect()
    }

    #[test]
    fn incompressible_full_chunks_are_stored_raw() {
        let data = noise(CHUNK_SIZE * 2, 0x1234_5678_9ABC_DEF1);
        let packed = compress(&data);
        // Both chunks raw: signature + 2 * (header + 4096).
        assert_eq!(packed.len(), 1 + 2 * (2 + CHUNK_SIZE));
        assert_eq!(&packed[1..3], &[0xFF, 0x3F]);
        assert_eq!(decompress(&packed, LIMIT).unwrap(), data);
    }

    #[test]
    fn short_incompressible_input_round_trips_exactly() {
        for len in [1, 2, 3, 7, 8, 9, 100, 1000, 3000, 3600] {
            let data = noise(len, len as u64 + 77);
            assert_eq!(decompress(&compress(&data), LIMIT).unwrap(), data, "{len}");
        }
    }

    #[test]
    fn raw_final_chunk_is_zero_padded() {
        // §2.4.1.3.10: a short incompressible final chunk that does not fit
        // once expanded is stored raw and padded with zeros.
        let data = noise(4000, 99);
        let out = decompress(&compress(&data), LIMIT).unwrap();
        assert_eq!(out.len(), CHUNK_SIZE);
        assert_eq!(&out[..4000], &data[..]);
        assert!(out[4000..].iter().all(|&b| b == 0));
    }

    /// Source-like text: compressible with copies at many distances.
    fn text(len: usize) -> Vec<u8> {
        let line = b"Public Sub Example(ByVal x As Long)\r\n    Dim total As Long: total = total + x\r\nEnd Sub\r\n";
        let mut v = Vec::new();
        let mut i = 0usize;
        while v.len() < len {
            v.extend_from_slice(line);
            v.extend_from_slice(format!("' comment {i}\r\n").as_bytes());
            i += 1;
        }
        v.truncate(len);
        v
    }

    #[test]
    fn round_trip_varied_sizes() {
        let sizes = [
            0, 1, 2, 3, 4, 15, 16, 17, 31, 32, 33, 64, 65, 1000, 4095, 4096, 4097, 8191, 8192,
            8193, 12288, 20000, 40_000,
        ];
        for &len in &sizes {
            let data = text(len);
            let packed = compress(&data);
            assert_eq!(decompress(&packed, LIMIT).unwrap(), data, "text {len}");
            let data = vec![0u8; len];
            let packed = compress(&data);
            assert_eq!(decompress(&packed, LIMIT).unwrap(), data, "zeros {len}");
        }
    }

    #[test]
    fn large_noise_round_trips_up_to_final_chunk_padding() {
        for len in [4096, 4097, 5000, 10_000, 12_288, 40_000] {
            let data = noise(len, len as u64);
            let out = decompress(&compress(&data), LIMIT).unwrap();
            assert!(out.len() >= data.len());
            assert_eq!(&out[..data.len()], &data[..], "{len}");
            assert!(out[data.len()..].iter().all(|&b| b == 0), "{len}");
            // Padding never exceeds one chunk.
            assert!(out.len() - data.len() < CHUNK_SIZE);
        }
    }

    #[test]
    fn mixed_content_round_trips() {
        let mut data = text(6000);
        data.extend(noise(3000, 5));
        data.extend(text(5000));
        data.extend(vec![0xFF; 9000]);
        let packed = compress(&data);
        assert_eq!(decompress(&packed, LIMIT).unwrap(), data);
    }

    #[test]
    fn compression_is_deterministic_and_shrinks_text() {
        let data = text(30_000);
        let a = compress(&data);
        assert_eq!(a, compress(&data));
        assert!(a.len() < data.len() / 2, "{} vs {}", a.len(), data.len());
    }

    #[test]
    fn hostile_input_never_panics() {
        // Random garbage and single-byte mutations of valid containers must
        // return Ok or an error, never panic or exceed the limit.
        let valid = compress(&text(9000));
        for seed in 0..300u64 {
            let mut data = noise(1 + (seed as usize * 7) % 300, seed + 1);
            data[0] = SIGNATURE_BYTE;
            if let Ok(out) = decompress(&data, 1 << 16) {
                assert!(out.len() <= 1 << 16);
            }
            let mut mutated = valid.clone();
            let pos = (seed as usize * 131) % mutated.len();
            mutated[pos] ^= (seed as u8) | 1;
            if let Ok(out) = decompress(&mutated, 1 << 16) {
                assert!(out.len() <= 1 << 16);
            }
            mutated.truncate(pos);
            let _ = decompress(&mutated, 1 << 16);
        }
    }

    #[test]
    fn every_chunk_header_is_well_formed() {
        let data = text(50_000);
        let packed = compress(&data);
        let mut pos = 1;
        while pos < packed.len() {
            let h = u16::from_le_bytes([packed[pos], packed[pos + 1]]);
            assert_eq!((h >> 12) & 7, 0b011);
            pos += usize::from(h & 0x0FFF) + 3;
        }
        assert_eq!(pos, packed.len());
    }

    #[test]
    fn literals_and_overlapping_copy() {
        // Hand-assembled: literals "abc", then copy (offset 3, length 9).
        let packed = [0x01, 0x05, 0xB0, 0x08, b'a', b'b', b'c', 0x06, 0x20];
        assert_eq!(decompress(&packed, LIMIT).unwrap(), b"abcabcabcabc");
    }

    #[test]
    fn uncompressed_chunk() {
        let mut packed = vec![0x01, 0xFF, 0x3F]; // signature 0b011, size field 4095
        packed.extend((0..CHUNK_SIZE).map(|i| i as u8));
        let out = decompress(&packed, LIMIT).unwrap();
        assert_eq!(out.len(), CHUNK_SIZE);
        assert_eq!(out[300], (300 % 256) as u8);
    }

    #[test]
    fn signature_only_is_empty() {
        assert_eq!(decompress(&[0x01], LIMIT).unwrap(), b"");
    }

    #[test]
    fn rejects_bad_signature() {
        assert_eq!(decompress(&[], LIMIT), Err(DecompressError::BadSignature));
        assert_eq!(
            decompress(&[0x02], LIMIT),
            Err(DecompressError::BadSignature)
        );
    }

    #[test]
    fn rejects_bad_chunk_signature_bits() {
        assert_eq!(
            decompress(&[0x01, 0x00, 0x80], LIMIT),
            Err(DecompressError::BadChunkHeader { offset: 1 })
        );
        assert_eq!(
            decompress(&[0x01, 0x05], LIMIT),
            Err(DecompressError::BadChunkHeader { offset: 1 })
        );
    }

    #[test]
    fn rejects_truncated_chunk() {
        let packed = [0x01, 0x05, 0xB0, 0x08, b'a'];
        assert_eq!(
            decompress(&packed, LIMIT),
            Err(DecompressError::TruncatedChunk { offset: 1 })
        );
    }

    #[test]
    fn rejects_copy_before_chunk_start() {
        // Flag says first token is a copy, but nothing has been produced.
        let packed = [0x01, 0x02, 0xB0, 0x01, 0x00, 0x00];
        assert!(matches!(
            decompress(&packed, LIMIT),
            Err(DecompressError::BadCopyOffset { .. })
        ));
    }

    #[test]
    fn rejects_offset_beyond_produced() {
        // Literal "a", then copy with offset 2 (only 1 byte available).
        let packed = [0x01, 0x03, 0xB0, 0x02, b'a', 0x00, 0x10];
        assert!(matches!(
            decompress(&packed, LIMIT),
            Err(DecompressError::BadCopyOffset { .. })
        ));
    }

    #[test]
    fn rejects_split_copy_token() {
        let packed = [0x01, 0x02, 0xB0, 0x02, b'a', 0x00];
        assert!(matches!(
            decompress(&packed, LIMIT),
            Err(DecompressError::MalformedTokens { .. })
        ));
    }

    #[test]
    fn enforces_output_limit() {
        let packed = [0x01, 0x05, 0xB0, 0x08, b'a', b'b', b'c', 0x06, 0x20];
        assert_eq!(
            decompress(&packed, 8),
            Err(DecompressError::LimitExceeded { limit: 8 })
        );
    }

    #[test]
    fn multiple_chunks_concatenate() {
        let one = [0x05, 0xB0, 0x08, b'a', b'b', b'c', 0x06, 0x20];
        let mut packed = vec![0x01];
        packed.extend(one);
        packed.extend(one);
        assert_eq!(
            decompress(&packed, LIMIT).unwrap(),
            b"abcabcabcabcabcabcabcabc"
        );
    }
}
