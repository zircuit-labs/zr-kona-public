use alloc::vec::Vec;
use alloy_primitives::Bytes;
use alloy_rlp::Buf;
use thiserror::Error;

const WORD_BYTES: usize = core::mem::size_of::<u64>();
const BITS_PER_WORD: usize = WORD_BYTES * 8;

#[derive(Debug)]
pub(crate) struct BitSet {
    length: usize,
    set: Vec<u64>,
}

impl BitSet {
    pub(super) fn contains(&self, bit: usize) -> bool {
        let idx = bit >> 6;

        bit < self.length && idx < self.set.len() && (self.set[idx] & (1u64 << (bit & 63))) != 0
    }
}

impl TryFrom<&Bytes> for BitSet {
    type Error = BitSetError;

    fn try_from(value: &Bytes) -> Result<Self, Self::Error> {
        // Need at least 8 bytes for the length header.
        if value.len() < WORD_BYTES {
            return Err(BitSetError::TooShortForHeader);
        }

        // Read big-endian length from the first 8 bytes.
        let mut hdr = value.slice(0..WORD_BYTES);
        let length = hdr.get_u64() as usize;

        // The remainder should be a whole number of u64 words.
        let rest = value.slice(WORD_BYTES..);
        let rem = rest.len() % WORD_BYTES;
        if rem != 0 {
            return Err(BitSetError::MisalignedWords(rem));
        }

        let words: Vec<u64> = rest
            .chunks_exact(WORD_BYTES)
            .map(|x| u64::from_be_bytes(x.try_into().expect("exact chunk of 8")))
            .collect();

        let needed_words = (length + (BITS_PER_WORD - 1)) / BITS_PER_WORD;
        if needed_words > words.len() {
            return Err(BitSetError::InconsistentLength { length, words: words.len() });
        }

        Ok(Self { length, set: words })
    }
}

/// BitSet Error
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BitSetError {
    /// Input had fewer than 8 bytes, so we couldn’t read the length header.
    #[error("input too short to contain length header (needs {WORD_BYTES} bytes)")]
    TooShortForHeader,

    /// Bytes after the header weren’t a multiple of 8 (word misalignment).
    #[error("bitset data not aligned to {WORD_BYTES}-byte words: {0} extra bytes")]
    MisalignedWords(usize),

    /// Declared length requires more words than provided in the payload.
    #[error("declared length {length} bits requires more words than provided ({words})")]
    InconsistentLength {
        /// Parsed length
        length: usize,

        /// Words length
        words: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::TryFrom;

    /// Helper to serialize a BitSet as:
    /// [8-byte big-endian length][8-byte big-endian words...]
    fn ser(length: usize, words: &[u64]) -> Bytes {
        let mut v = Vec::with_capacity(WORD_BYTES + words.len() * WORD_BYTES);
        v.extend_from_slice(&(length as u64).to_be_bytes());
        for &w in words {
            v.extend_from_slice(&w.to_be_bytes());
        }
        Bytes::from(v)
    }

    #[test]
    fn contains_basic_true_false() {
        let words = [
            1u64,      // bit 0 set
            0u64,      // empty middle word
            1u64 << 1, // bit 1 set
        ];
        let b = BitSet::try_from(&ser(130, &words)).expect("valid bitset");

        assert!(b.contains(0));
        assert!(b.contains(129));

        assert!(!b.contains(1));
        assert!(!b.contains(128));
        assert!(!b.contains(130));
        assert!(!b.contains(10_000));
    }

    #[test]
    fn out_of_range_returns_false_even_if_word_exists() {
        let words = [0u64, 0xFFFF_FFFF_FFFF_FFFFu64];
        let b = BitSet::try_from(&ser(64, &words)).expect("valid bitset");

        // Any bit >= length must return false, even if the extra word has bits set.
        assert!(!b.contains(64));
        assert!(!b.contains(127));
    }

    #[test]
    fn zero_length_ok_all_false() {
        let words = [0xFFFFu64]; // words may exist, but length=0 says all queries are out of range
        let b = BitSet::try_from(&ser(0, &words)).expect("valid bitset");
        assert!(!b.contains(0));
        assert!(!b.contains(63));
        assert!(!b.contains(10_000));
    }

    #[test]
    fn inconsistent_length_errors_when_words_too_few() {
        // length = 129 -> needs 3 words, but provide only 2.
        let words = [0u64, 0u64];
        let err = BitSet::try_from(&ser(129, &words)).unwrap_err();
        assert!(matches!(err, BitSetError::InconsistentLength { .. }));
        if let BitSetError::InconsistentLength { length, words } = err {
            assert_eq!(length, 129);
            assert_eq!(words, 2);
        }
    }

    #[test]
    fn trailing_bytes_error() {
        let mut raw = ser(64, &[0u64; 1]).0.to_vec();
        raw.push(0xAA);
        let raw = Bytes::from(raw);
        let err = BitSet::try_from(&Bytes::from(raw)).unwrap_err();
        assert!(matches!(err, BitSetError::MisalignedWords(1)));
    }

    #[test]
    fn too_short_for_header_error() {
        let v = alloc::vec![0u8; WORD_BYTES - 1];
        let err = BitSet::try_from(&Bytes::from(v)).unwrap_err();
        assert!(matches!(err, BitSetError::TooShortForHeader));
    }
}
