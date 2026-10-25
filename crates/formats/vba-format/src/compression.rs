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

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: usize = 1 << 20;

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
