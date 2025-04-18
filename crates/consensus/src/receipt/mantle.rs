//! Transaction receipt types for Optimism.

use alloy_consensus::{
    Eip658Value, Receipt, ReceiptWithBloom, RlpDecodableReceipt, RlpEncodableReceipt, TxReceipt,
};
use alloy_primitives::{Bloom, Log};
use alloy_rlp::{Buf, BufMut, Decodable, Encodable, Header};

/// Store the L1 info and the token ratio for future use in the receipt.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MantleTxStoredReceipt<T = Log> {
    /// The inner receipt type.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: Receipt<T>,
    /// The base fee of the L1 origin block.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub l1_base_fee: Option<u128>,
    /// The current L1 fee overhead. None if Ecotone is activated.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub l1_fee_overhead: Option<u128>,
    /// The current L1 fee overhead. None if Ecotone is activated.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub l1_base_fee_scalar: Option<u128>,
    /* ---------------------------------------- Mantle ---------------------------------------- */
    /// token ratio between eth and mnt
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub token_ratio: Option<u128>,
}

impl MantleTxStoredReceipt {
    /// Calculates [`Log`]'s bloom filter. this is slow operation and
    /// [MantleTxStoredReceiptWithBloom] can be used to cache this value.
    pub fn bloom_slow(&self) -> Bloom {
        self.inner.logs.iter().collect()
    }

    /// Calculates the bloom filter for the receipt and returns the [MantleTxStoredReceiptWithBloom]
    /// container type.
    pub fn with_bloom(self) -> MantleTxStoredReceiptWithBloom {
        self.into()
    }
}

impl<T: Encodable> MantleTxStoredReceipt<T> {
    /// Returns length of RLP-encoded receipt fields with the given [`Bloom`] without an RLP header.
    pub fn rlp_encoded_fields_length_with_bloom(&self, bloom: &Bloom) -> usize {
        self.inner.rlp_encoded_fields_length_with_bloom(bloom)
            + self.l1_base_fee.map_or(0, |price| price.length())
            + self.l1_fee_overhead.map_or(0, |used| used.length())
            + self.l1_base_fee_scalar.map_or(0, |fee| fee.length())
            + self.token_ratio.map_or(0, |ratio| ratio.length())
    }

    /// RLP-encodes receipt fields with the given [`Bloom`] without an RLP header.
    pub fn rlp_encode_fields_with_bloom(&self, bloom: &Bloom, out: &mut dyn BufMut) {
        self.inner.rlp_encode_fields_with_bloom(bloom, out);

        if let Some(price) = self.l1_base_fee {
            price.encode(out);
        }
        if let Some(used) = self.l1_fee_overhead {
            used.encode(out);
        }
        if let Some(fee) = self.l1_base_fee_scalar {
            fee.encode(out);
        }
        if let Some(ratio) = self.token_ratio {
            ratio.encode(out);
        }
    }

    /// Returns RLP header for this receipt encoding with the given [`Bloom`].
    pub fn rlp_header_with_bloom(&self, bloom: &Bloom) -> Header {
        Header { list: true, payload_length: self.rlp_encoded_fields_length_with_bloom(bloom) }
    }
}

impl<T: Decodable> MantleTxStoredReceipt<T> {
    /// RLP-decodes receipt's field with a [`Bloom`].
    ///
    /// Does not expect an RLP header.
    pub fn rlp_decode_fields_with_bloom(
        buf: &mut &[u8],
    ) -> alloy_rlp::Result<ReceiptWithBloom<Self>> {
        let ReceiptWithBloom { receipt: inner, logs_bloom } =
            Receipt::rlp_decode_fields_with_bloom(buf)?;

        let l1_base_fee = (!buf.is_empty()).then(|| Decodable::decode(buf)).transpose()?;
        let l1_fee_overhead = (!buf.is_empty()).then(|| Decodable::decode(buf)).transpose()?;
        let l1_base_fee_scalar = (!buf.is_empty()).then(|| Decodable::decode(buf)).transpose()?;
        let token_ratio = (!buf.is_empty()).then(|| Decodable::decode(buf)).transpose()?;

        Ok(ReceiptWithBloom {
            logs_bloom,
            receipt: Self {
                inner,
                l1_base_fee,
                l1_fee_overhead,
                l1_base_fee_scalar,
                token_ratio,
            },
        })
    }
}

impl<T> AsRef<Receipt<T>> for MantleTxStoredReceipt<T> {
    fn as_ref(&self) -> &Receipt<T> {
        &self.inner
    }
}

impl<T> TxReceipt for MantleTxStoredReceipt<T>
where
    T: AsRef<Log> + Clone + core::fmt::Debug + PartialEq + Eq + Send + Sync,
{
    type Log = T;

    fn status_or_post_state(&self) -> Eip658Value {
        self.inner.status_or_post_state()
    }

    fn status(&self) -> bool {
        self.inner.status()
    }

    fn bloom(&self) -> Bloom {
        self.inner.bloom_slow()
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.inner.cumulative_gas_used()
    }

    fn logs(&self) -> &[Self::Log] {
        self.inner.logs()
    }
}

impl<T: Encodable> RlpEncodableReceipt for MantleTxStoredReceipt<T> {
    fn rlp_encoded_length_with_bloom(&self, bloom: &Bloom) -> usize {
        self.rlp_header_with_bloom(bloom).length_with_payload()
    }

    fn rlp_encode_with_bloom(&self, bloom: &Bloom, out: &mut dyn BufMut) {
        self.rlp_header_with_bloom(bloom).encode(out);
        self.rlp_encode_fields_with_bloom(bloom, out);
    }
}

impl<T: Decodable> RlpDecodableReceipt for MantleTxStoredReceipt<T> {
    fn rlp_decode_with_bloom(buf: &mut &[u8]) -> alloy_rlp::Result<ReceiptWithBloom<Self>> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        if buf.len() < header.payload_length {
            return Err(alloy_rlp::Error::InputTooShort);
        }

        // Note: we pass a separate buffer to `rlp_decode_fields_with_bloom` to allow it decode
        // optional fields based on the remaining length.
        let mut fields_buf = &buf[..header.payload_length];
        let this = Self::rlp_decode_fields_with_bloom(&mut fields_buf)?;

        if !fields_buf.is_empty() {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        buf.advance(header.payload_length);

        Ok(this)
    }
}


/// [`MantleTxStoredReceipt`] with calculated bloom filter, modified for the OP Stack.
///
/// This convenience type allows us to lazily calculate the bloom filter for a
/// receipt, similar to [`Sealed`].
///
/// [`Sealed`]: alloy_consensus::Sealed
pub type MantleTxStoredReceiptWithBloom<T = Log> = ReceiptWithBloom<MantleTxStoredReceipt<T>>;

#[cfg(feature = "arbitrary")]
impl<'a, T> arbitrary::Arbitrary<'a> for MantleTxStoredReceipt<T>
where
    T: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        use alloc::vec::Vec;
        let l1_base_fee = Option::<u128>::arbitrary(u)?;
        let l1_fee_overhead = Option::<u128>::arbitrary(u)?;
        let l1_base_fee_scalar = Option::<u128>::arbitrary(u)?;
        let token_ratio = Option::<u128>::arbitrary(u)?;

        Ok(Self {
            inner: Receipt {
                status: Eip658Value::arbitrary(u)?,
                cumulative_gas_used: u64::arbitrary(u)?,
                logs: Vec::<T>::arbitrary(u)?,
            },
            l1_base_fee,
            l1_fee_overhead,
            l1_base_fee_scalar,
            token_ratio,
        })
    }
}

/// Bincode-compatible [`MantleTxStoredReceipt`] serde implementation.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(crate) mod serde_bincode_compat {
    use alloc::{borrow::Cow, vec::Vec};
    use alloy_consensus::Receipt;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Bincode-compatible [`super::MantleTxStoredReceipt`] serde implementation.
    ///
    /// Intended to use with the [`serde_with::serde_as`] macro in the following way:
    /// ```rust
    /// use op_alloy_consensus::{serde_bincode_compat, MantleTxStoredReceipt};
    /// use serde::{de::DeserializeOwned, Deserialize, Serialize};
    /// use serde_with::serde_as;
    ///
    /// #[serde_as]
    /// #[derive(Serialize, Deserialize)]
    /// struct Data<T: Serialize + DeserializeOwned + Clone + 'static> {
    ///     #[serde_as(as = "serde_bincode_compat::MantleTxStoredReceipt<'_, T>")]
    ///     receipt: MantleTxStoredReceipt<T>,
    /// }
    /// ```
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MantleTxStoredReceipt<'a, T: Clone> {
        logs: Cow<'a, Vec<T>>,
        status: bool,
        cumulative_gas_used: u64,
        l1_base_fee: Option<u128>,
        l1_fee_overhead: Option<u128>,
        l1_base_fee_scalar: Option<u128>,
        token_ratio: Option<u128>,
    }

    impl<'a, T: Clone> From<&'a super::MantleTxStoredReceipt<T>> for MantleTxStoredReceipt<'a, T> {
        fn from(value: &'a super::MantleTxStoredReceipt<T>) -> Self {
            Self {
                logs: Cow::Borrowed(&value.inner.logs),
                status: value.inner.status.coerce_status(),
                cumulative_gas_used: value.inner.cumulative_gas_used,
                l1_base_fee: value.l1_base_fee,
                l1_fee_overhead: value.l1_fee_overhead,
                l1_base_fee_scalar: value.l1_base_fee_scalar,
                token_ratio: value.token_ratio,
            }
        }
    }

    impl<'a, T: Clone> From<MantleTxStoredReceipt<'a, T>> for super::MantleTxStoredReceipt<T> {
        fn from(value: MantleTxStoredReceipt<'a, T>) -> Self {
            Self {
                inner: Receipt {
                    status: value.status.into(),
                    cumulative_gas_used: value.cumulative_gas_used,
                    logs: value.logs.into_owned(),
                },
                l1_base_fee: value.l1_base_fee,
                l1_fee_overhead: value.l1_fee_overhead,
                l1_base_fee_scalar: value.l1_base_fee_scalar,
                token_ratio: value.token_ratio,
            }
        }
    }

    impl<T: Serialize + Clone> SerializeAs<super::MantleTxStoredReceipt<T>>
        for MantleTxStoredReceipt<'_, T>
    {
        fn serialize_as<S>(
            source: &super::MantleTxStoredReceipt<T>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            MantleTxStoredReceipt::<'_, T>::from(source).serialize(serializer)
        }
    }

    impl<'de, T: Deserialize<'de> + Clone> DeserializeAs<'de, super::MantleTxStoredReceipt<T>>
        for MantleTxStoredReceipt<'de, T>
    {
        fn deserialize_as<D>(deserializer: D) -> Result<super::MantleTxStoredReceipt<T>, D::Error>
        where
            D: Deserializer<'de>,
        {
            MantleTxStoredReceipt::<'_, T>::deserialize(deserializer).map(Into::into)
        }
    }
    #[cfg(test)]
    mod tests {
        use super::super::{serde_bincode_compat, MantleTxStoredReceipt};
        use alloy_primitives::Log;
        use arbitrary::Arbitrary;
        use rand::Rng;
        use serde::{de::DeserializeOwned, Deserialize, Serialize};
        use serde_with::serde_as;

        #[test]
        fn test_tx_bincode_roundtrip() {
            #[serde_as]
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Data<T: Serialize + DeserializeOwned + Clone + 'static> {
                #[serde_as(as = "serde_bincode_compat::MantleTxStoredReceipt<'_,T>")]
                transaction: MantleTxStoredReceipt<T>,
            }

            let mut bytes = [0u8; 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());
            let data = Data {
                transaction: MantleTxStoredReceipt::arbitrary(&mut arbitrary::Unstructured::new(
                    &bytes,
                ))
                .unwrap(),
            };

            let encoded = bincode::serialize(&data).unwrap();
            let decoded: Data<Log> = bincode::deserialize(&encoded).unwrap();
            assert_eq!(decoded, data);
        }
    }
}


// [TODO] add tests for the new fields
