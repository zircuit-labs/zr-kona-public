//! Contains ecotone-specific L1 block info types.

use crate::DecodeError;
use alloc::vec::Vec;
use alloy_primitives::{Address, B256, Bytes, U256};

/// Represents the fields within an Ecotone L1 block info transaction.
///
/// Ecotone Binary Format
/// +---------+--------------------------+
/// | Bytes   | Field                    |
/// +---------+--------------------------+
/// | 4       | Function signature       |
/// | 4       | BaseFeeScalar            |
/// | 4       | BlobBaseFeeScalar        |
/// | 8       | SequenceNumber           |
/// | 8       | Timestamp                |
/// | 8       | L1BlockNumber            |
/// | 32      | BaseFee                  |
/// | 32      | BlobBaseFee              |
/// | 32      | BlockHash                |
/// | 32      | BatcherHash              |
/// | 32      | DepositExclusionLen      |
/// | N       | ExclusionsData           |
/// +---------+--------------------------+
#[derive(Debug, Clone, Hash, Eq, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct L1BlockInfoEcotone {
    /// The current L1 origin block number
    pub number: u64,
    /// The current L1 origin block's timestamp
    pub time: u64,
    /// The current L1 origin block's basefee
    pub base_fee: u64,
    /// The current L1 origin block's hash
    pub block_hash: B256,
    /// The current sequence number
    pub sequence_number: u64,
    /// The address of the batch submitter
    pub batcher_address: Address,
    /// The current blob base fee on L1
    pub blob_base_fee: u128,
    /// The fee scalar for L1 blobspace data
    pub blob_base_fee_scalar: u32,
    /// The fee scalar for L1 data
    pub base_fee_scalar: u32,
    /// Indicates that the scalars are empty.
    /// This is an edge case where the first block in ecotone has no scalars,
    /// so the bedrock tx l1 cost function needs to be used.
    pub empty_scalars: bool,
    /// The l1 fee overhead used along with the `empty_scalars` field for the
    /// bedrock tx l1 cost function.
    ///
    /// This field is deprecated in the Ecotone Hardfork.
    pub l1_fee_overhead: U256,
    /// Deposit exclusions bytes
    pub deposit_exclusions: Option<Bytes>,
}

impl L1BlockInfoEcotone {
    /// The type byte identifier for the L1 scalar format in Ecotone.
    pub const L1_SCALAR: u8 = 1;

    /// The length of an L1 info transaction in Ecotone.
    pub const L1_INFO_TX_LEN: usize = 4 + 32 * 5;

    /// The minimum length of an L1 info transaction in Ecotone with deposit exclusions.
    pub const L1_INFO_EXCLUSIONS_TX_MIN_LEN: usize = Self::L1_INFO_TX_LEN + U256::BYTES;

    /// The 4 byte selector of "setL1BlockValuesEcotone()"
    pub const L1_INFO_TX_SELECTOR: [u8; 4] = [0x44, 0x0a, 0x5e, 0x20];

    /// The 4 byte selector of "setL1BlockValuesEcotoneExclusions"
    pub const L1_INFO_TX_EXCLUSIONS_SELECTOR: [u8; 4] = [0xcb, 0x2d, 0x34, 0x3f];

    /// Encodes the [`L1BlockInfoEcotone`] object into Ethereum transaction calldata.
    pub fn encode_calldata(&self) -> Bytes {
        let mut buf = Vec::with_capacity(
            self.deposit_exclusions
                .as_ref()
                .map_or(Self::L1_INFO_TX_LEN, |d| d.len() + Self::L1_INFO_EXCLUSIONS_TX_MIN_LEN),
        );
        if self.deposit_exclusions.is_some() {
            buf.extend_from_slice(Self::L1_INFO_TX_EXCLUSIONS_SELECTOR.as_ref());
        } else {
            buf.extend_from_slice(Self::L1_INFO_TX_SELECTOR.as_ref());
        }
        buf.extend_from_slice(self.base_fee_scalar.to_be_bytes().as_ref());
        buf.extend_from_slice(self.blob_base_fee_scalar.to_be_bytes().as_ref());
        buf.extend_from_slice(self.sequence_number.to_be_bytes().as_ref());
        buf.extend_from_slice(self.time.to_be_bytes().as_ref());
        buf.extend_from_slice(self.number.to_be_bytes().as_ref());
        buf.extend_from_slice(U256::from(self.base_fee).to_be_bytes::<{ U256::BYTES }>().as_ref());
        buf.extend_from_slice(
            U256::from(self.blob_base_fee).to_be_bytes::<{ U256::BYTES }>().as_ref(),
        );
        buf.extend_from_slice(self.block_hash.as_ref());
        buf.extend_from_slice(self.batcher_address.into_word().as_ref());
        if let Some(deposit_exclusions) = &self.deposit_exclusions {
            buf.extend_from_slice(
                U256::from(deposit_exclusions.len()).to_be_bytes::<{ U256::BYTES }>().as_ref(),
            );
            buf.extend_from_slice(deposit_exclusions);
        }
        // Notice: do not include the `empty_scalars` field in the calldata.
        // Notice: do not include the `l1_fee_overhead` field in the calldata.
        buf.into()
    }

    /// Decodes the [`L1BlockInfoEcotone`] object from ethereum transaction calldata.
    pub fn decode_calldata(r: &[u8]) -> Result<Self, DecodeError> {
        const SELECTOR_LEN: usize = 4;
        if r.len() < SELECTOR_LEN {
            return Err(DecodeError::InvalidEcotoneLength(SELECTOR_LEN, r.len()));
        }

        let selector: [u8; SELECTOR_LEN] = r[0..SELECTOR_LEN].try_into().unwrap(); // SAFETY: slice length is validated above

        match selector {
            Self::L1_INFO_TX_SELECTOR => {
                if r.len() != Self::L1_INFO_TX_LEN {
                    return Err(DecodeError::InvalidEcotoneLength(Self::L1_INFO_TX_LEN, r.len()));
                }
            }
            Self::L1_INFO_TX_EXCLUSIONS_SELECTOR => {
                if r.len() < Self::L1_INFO_EXCLUSIONS_TX_MIN_LEN {
                    return Err(DecodeError::InvalidEcotoneLength(
                        Self::L1_INFO_EXCLUSIONS_TX_MIN_LEN,
                        r.len(),
                    ));
                }

                let len_bytes: [u8; U256::BYTES] = r
                    [Self::L1_INFO_TX_LEN..Self::L1_INFO_EXCLUSIONS_TX_MIN_LEN]
                    .try_into()
                    .unwrap(); // SAFETY: slice length is validated above

                let deposit_length = U256::from_be_bytes(len_bytes).to::<usize>();

                if r.len() != Self::L1_INFO_EXCLUSIONS_TX_MIN_LEN + deposit_length {
                    return Err(DecodeError::InvalidEcotoneLength(
                        Self::L1_INFO_EXCLUSIONS_TX_MIN_LEN + deposit_length,
                        r.len(),
                    ));
                }
            }
            _ => {
                return Err(DecodeError::InvalidSelector);
            }
        }
        // SAFETY: For all below slice operations, the full
        //         length is validated above to be `164`.

        // SAFETY: 4 bytes are copied directly into the array
        let mut base_fee_scalar = [0u8; 4];
        base_fee_scalar.copy_from_slice(&r[4..8]);
        let base_fee_scalar = u32::from_be_bytes(base_fee_scalar);

        // SAFETY: 4 bytes are copied directly into the array
        let mut blob_base_fee_scalar = [0u8; 4];
        blob_base_fee_scalar.copy_from_slice(&r[8..12]);
        let blob_base_fee_scalar = u32::from_be_bytes(blob_base_fee_scalar);

        // SAFETY: 8 bytes are copied directly into the array
        let mut sequence_number = [0u8; 8];
        sequence_number.copy_from_slice(&r[12..20]);
        let sequence_number = u64::from_be_bytes(sequence_number);

        // SAFETY: 8 bytes are copied directly into the array
        let mut time = [0u8; 8];
        time.copy_from_slice(&r[20..28]);
        let time = u64::from_be_bytes(time);

        // SAFETY: 8 bytes are copied directly into the array
        let mut number = [0u8; 8];
        number.copy_from_slice(&r[28..36]);
        let number = u64::from_be_bytes(number);

        // SAFETY: 8 bytes are copied directly into the array
        let mut base_fee = [0u8; 8];
        base_fee.copy_from_slice(&r[60..68]);
        let base_fee = u64::from_be_bytes(base_fee);

        // SAFETY: 16 bytes are copied directly into the array
        let mut blob_base_fee = [0u8; 16];
        blob_base_fee.copy_from_slice(&r[84..100]);
        let blob_base_fee = u128::from_be_bytes(blob_base_fee);

        let block_hash = B256::from_slice(r[100..132].as_ref());
        let batcher_address = Address::from_slice(r[144..164].as_ref());

        let deposit_exclusions = (selector == Self::L1_INFO_TX_EXCLUSIONS_SELECTOR)
            .then(|| Bytes::copy_from_slice(&r[Self::L1_INFO_EXCLUSIONS_TX_MIN_LEN..]));

        Ok(Self {
            number,
            time,
            base_fee,
            block_hash,
            sequence_number,
            batcher_address,
            blob_base_fee,
            blob_base_fee_scalar,
            base_fee_scalar,
            // Notice: the `empty_scalars` field is not included in the calldata.
            // This is used by the evm to indicate that the bedrock tx l1 cost function
            // needs to be used.
            empty_scalars: false,
            // Notice: the `l1_fee_overhead` field is not included in the calldata.
            l1_fee_overhead: U256::ZERO,
            deposit_exclusions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    const ENCODED_ECOTONE_L1_INFO: &str = "cb2d343f00000315000000ea000000000000b26e00000000664576b6000000000000007b0000000000000000000000000000000000000000000000000000000000ab4130000000000000000000000000000000000000000000000000000000000000270436cb48eea9f188c92cbe4507062cde28f9da66b690ea76f1dc7b725896878313000000000000000000000000492c193f068b11ee52a017577805188e45af517a000000000000000000000000000000000000000000000000000000000000001000000000000000050000000000000004";

    // An encoded bitmap for deposit exclusion
    // the bitset is encoded with 8 bytes for the length and a slice of u64s, depending on the length
    // 0000000000000005|0000000000000004
    const ENCODED_EXCLUSION_DATA: &str = "00000000000000050000000000000004";

    #[test]
    fn test_decode_calldata_ecotone_invalid_length() {
        let r = vec![0u8; 1];
        assert_eq!(
            L1BlockInfoEcotone::decode_calldata(&r),
            Err(DecodeError::InvalidEcotoneLength(4, r.len(),))
        );
    }

    #[test]
    fn test_l1_block_info_ecotone_roundtrip_calldata_encoding() {
        let info = L1BlockInfoEcotone {
            number: 1,
            time: 2,
            base_fee: 3,
            block_hash: B256::from([4u8; 32]),
            sequence_number: 5,
            batcher_address: Address::from([6u8; 20]),
            blob_base_fee: 7,
            blob_base_fee_scalar: 8,
            base_fee_scalar: 9,
            empty_scalars: false,
            l1_fee_overhead: U256::ZERO,
            deposit_exclusions: None,
        };

        let calldata = info.encode_calldata();
        let decoded_info = L1BlockInfoEcotone::decode_calldata(&calldata).unwrap();
        assert_eq!(info, decoded_info);
    }

    #[test]
    fn test_decode_calldata_ecotone_exclusions_invalid_length() {
        // Test with length less than minimum
        let mut r = vec![0u8; L1BlockInfoEcotone::L1_INFO_EXCLUSIONS_TX_MIN_LEN - 1];
        r[0..4].copy_from_slice(&L1BlockInfoEcotone::L1_INFO_TX_EXCLUSIONS_SELECTOR);

        assert_eq!(
            L1BlockInfoEcotone::decode_calldata(&r),
            Err(DecodeError::InvalidEcotoneLength(
                L1BlockInfoEcotone::L1_INFO_EXCLUSIONS_TX_MIN_LEN,
                r.len()
            ))
        );

        let mut r = vec![0u8; L1BlockInfoEcotone::L1_INFO_EXCLUSIONS_TX_MIN_LEN + 10];
        r[0..4].copy_from_slice(&L1BlockInfoEcotone::L1_INFO_TX_EXCLUSIONS_SELECTOR);
        r[L1BlockInfoEcotone::L1_INFO_TX_LEN..L1BlockInfoEcotone::L1_INFO_EXCLUSIONS_TX_MIN_LEN]
            .copy_from_slice(&U256::from(20).to_be_bytes::<{ U256::BYTES }>());

        assert_eq!(
            L1BlockInfoEcotone::decode_calldata(&r),
            Err(DecodeError::InvalidEcotoneLength(
                L1BlockInfoEcotone::L1_INFO_EXCLUSIONS_TX_MIN_LEN + 20,
                r.len()
            ))
        );
    }

    #[test]
    fn test_l1_block_info_ecotone_with_exclusions() {
        let deposit_exclusions =
            Bytes::from(alloy_primitives::hex::decode(ENCODED_EXCLUSION_DATA).unwrap());
        let info = L1BlockInfoEcotone {
            number: 123,
            time: 1715828406,
            base_fee: 11223344,
            block_hash: B256::from_slice(
                &alloy_primitives::hex::decode(
                    "36cb48eea9f188c92cbe4507062cde28f9da66b690ea76f1dc7b725896878313",
                )
                .unwrap(),
            ),
            sequence_number: 45678,
            batcher_address: Address::from_slice(
                &alloy_primitives::hex::decode("492c193f068B11ee52a017577805188e45aF517a").unwrap(),
            ),
            blob_base_fee: 9988,
            blob_base_fee_scalar: 234,
            base_fee_scalar: 789,
            empty_scalars: false,
            l1_fee_overhead: U256::ZERO,
            deposit_exclusions: Some(deposit_exclusions.clone()),
        };

        let calldata = info.encode_calldata();

        // Verify it uses the exclusion selector
        assert_eq!(&calldata[0..4], L1BlockInfoEcotone::L1_INFO_TX_EXCLUSIONS_SELECTOR);
        assert_eq!(&calldata[4..8], &789u32.to_be_bytes()); // BaseFeeScalar
        assert_eq!(&calldata[8..12], &234u32.to_be_bytes()); // BlobBaseFeeScalar
        assert_eq!(&calldata[12..20], &45678u64.to_be_bytes()); // SequenceNumber
        assert_eq!(&calldata[20..28], &1715828406u64.to_be_bytes()); // Timestamp
        assert_eq!(&calldata[28..36], &123u64.to_be_bytes()); // L1BlockNumber

        let exclusions_len_offset = 164;
        let exclusions_len =
            U256::from_be_slice(&calldata[exclusions_len_offset..exclusions_len_offset + 32]);
        assert_eq!(exclusions_len, U256::from(deposit_exclusions.len()));

        let exclusions_data_start = exclusions_len_offset + 32;
        assert_eq!(&calldata[exclusions_data_start..], deposit_exclusions.as_ref());

        let expected_calldata = alloy_primitives::hex::decode(ENCODED_ECOTONE_L1_INFO).unwrap();
        assert_eq!(calldata, Bytes::from(expected_calldata));
    }

    #[test]
    fn test_encoding_decoding_roundtrip() {
        let calldata = alloy_primitives::hex::decode(ENCODED_ECOTONE_L1_INFO).unwrap();
        let decoded_l1_info = L1BlockInfoEcotone::decode_calldata(&calldata).unwrap();
        let re_encoded = decoded_l1_info.encode_calldata();
        assert_eq!(re_encoded.as_ref(), calldata.as_slice());
    }
}
