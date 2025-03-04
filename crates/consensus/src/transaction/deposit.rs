//! Deposit Transaction type.

use super::OpTxType;
use crate::DepositTransaction;
use alloc::vec::Vec;
use alloy_consensus::{Sealable, Transaction};
use alloy_eips::eip2930::AccessList;
use alloy_primitives::{
    keccak256, Address, Bytes, ChainId, PrimitiveSignature as Signature, TxHash, TxKind, B256, U256,
};
use alloy_rlp::{
    Buf, BufMut, Decodable, Encodable, Error as DecodeError, Header, EMPTY_STRING_CODE,
};
use core::mem;
use alloy_eips::Typed2718;

/// Deposit transactions, also known as deposits are initiated on L1, and executed on L2.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TxDeposit {
    /// Hash that uniquely identifies the source of the deposit.
    pub source_hash: B256,
    /// The address of the sender account.
    pub from: Address,
    /// The address of the recipient account, or the null (zero-length) address if the deposited
    /// transaction is a contract creation.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "TxKind::is_create"))]
    pub to: TxKind,
    /// The Mnt value to mint on L2.
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity::opt"))]
    pub mint: Option<u128>,
    ///  The Mnt value to send to the recipient account.
    pub value: U256,
    /// The gas limit for the L2 transaction.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity", rename = "gas"))]
    pub gas_limit: u64,
    /// Field indicating if this transaction is exempt from the L2 gas limit.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            with = "alloy_serde::quantity",
            rename = "isSystemTx",
            skip_serializing_if = "core::ops::Not::not"
        )
    )]
    pub is_system_transaction: bool,
    ///EthValue means L2 BVM_ETH mint tag, nil means that there is no need to mint BVM_ETH.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt", rename = "ethValue")
    )]
    pub eth_value: Option<u128>,
    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some).
    pub input: Bytes,
    /// EthTxValue means L2 BVM_ETH tx tag, nil means that there is no need to transfer BVM_ETH to
    /// msg.To.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt", rename = "ethTxValue")
    )]
    pub eth_tx_value: Option<u128>,
}

impl DepositTransaction for TxDeposit {
    fn source_hash(&self) -> Option<B256> {
        Some(self.source_hash)
    }

    fn mint(&self) -> Option<u128> {
        self.mint
    }

    fn is_system_transaction(&self) -> bool {
        self.is_system_transaction
    }

    fn is_deposit(&self) -> bool {
        true
    }
}

impl TxDeposit {
    /// Decodes the inner [TxDeposit] fields from RLP bytes.
    ///
    /// NOTE: This assumes a RLP header has already been decoded, and _just_ decodes the following
    /// RLP fields in the following order:
    ///
    /// - `source_hash`
    /// - `from`
    /// - `to`
    /// - `mint`
    /// - `value`
    /// - `gas_limit`
    /// - `is_system_transaction`
    /// - `eth_value`
    /// - `input`
    /// - `eth_tx_value`
    pub fn rlp_decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            source_hash: Decodable::decode(buf)?,
            from: Decodable::decode(buf)?,
            to: Decodable::decode(buf)?,
            mint: Self::decode_u128_from_rlp(buf)?,
            value: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            is_system_transaction: Decodable::decode(buf)?,
            eth_value: Self::decode_u128_from_rlp(buf)?,
            input: Decodable::decode(buf)?,
            eth_tx_value: Self::decode_optional_u128_from_rlp(buf)?,
        })
    }

    /// Decodes a u128 value from RLP format. If the value doesn't exist, returns nil.
    pub fn decode_u128_from_rlp(buf: &mut &[u8]) -> Result<Option<u128>, DecodeError> {
        if *buf.first().ok_or(DecodeError::InputTooShort)? == EMPTY_STRING_CODE {
            buf.advance(1);
            Ok(None)
        } else {
            Ok(Some(Decodable::decode(buf)?))
        }
    }

    /// Decodes a u128 value from RLP format. If the value doesn't exist, the field will be omitted
    /// from encoding.
    pub fn decode_optional_u128_from_rlp(buf: &mut &[u8]) -> Result<Option<u128>, DecodeError> {
        if buf.is_empty() {
            return Ok(None);
        }
        Self::decode_u128_from_rlp(buf)
    }

    /// Decodes the transaction from RLP bytes.
    pub fn rlp_decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        let remaining = buf.len();

        if header.payload_length > remaining {
            return Err(alloy_rlp::Error::InputTooShort);
        }

        let this = Self::rlp_decode_fields(buf)?;

        if buf.len() + header.payload_length != remaining {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        Ok(this)
    }

    /// Outputs the length of the transaction's fields, without a RLP header or length of the
    /// eip155 fields.
    pub(crate) fn rlp_encoded_fields_length(&self) -> usize {
        self.source_hash.length()
            + self.from.length()
            + self.to.length()
            + self.mint.map_or(1, |mint| mint.length())
            + self.value.length()
            + self.gas_limit.length()
            + self.is_system_transaction.length()
            + self.input.0.length()
            + self.eth_value.map_or(1, |eth| eth.length())
            + self.eth_tx_value.map_or(0, |eth| eth.length())
    }

    /// Encodes only the transaction's fields into the desired buffer, without a RLP header.
    /// <https://github.com/ethereum-optimism/specs/blob/main/specs/protocol/deposits.md#the-deposited-transaction-type>
    pub(crate) fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.source_hash.encode(out);
        self.from.encode(out);
        self.to.encode(out);
        if let Some(mint) = self.mint {
            mint.encode(out);
        } else {
            out.put_u8(EMPTY_STRING_CODE);
        }
        self.value.encode(out);
        self.gas_limit.encode(out);
        self.is_system_transaction.encode(out);
        if let Some(eth_value) = self.eth_value {
            eth_value.encode(out);
        } else {
            out.put_u8(EMPTY_STRING_CODE);
        }
        self.input.encode(out);
        if let Some(eth_tx_value) = self.eth_tx_value {
            eth_tx_value.encode(out);
        }
    }

    /// Calculates a heuristic for the in-memory size of the [TxDeposit] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<B256>() + // source_hash
        mem::size_of::<Address>() + // from
        self.to.size() + // to
        mem::size_of::<Option<u128>>() + // mint
        mem::size_of::<U256>() + // value
        mem::size_of::<u128>() + // gas_limit
        mem::size_of::<bool>() + // is_system_transaction
        self.input.len() + // input
        mem::size_of::<Option<u128>>() + //eth_value
        mem::size_of::<Option<u128>>() // eth_tx_value
    }

    /// Get the transaction type
    pub(crate) const fn tx_type(&self) -> OpTxType {
        OpTxType::Deposit
    }

    /// Create an rlp header for the transaction.
    fn rlp_header(&self) -> Header {
        Header { list: true, payload_length: self.rlp_encoded_fields_length() }
    }

    /// RLP encodes the transaction.
    pub fn rlp_encode(&self, out: &mut dyn BufMut) {
        self.rlp_header().encode(out);
        self.rlp_encode_fields(out);
    }

    /// Get the length of the transaction when RLP encoded.
    pub fn rlp_encoded_length(&self) -> usize {
        self.rlp_header().length_with_payload()
    }

    /// Get the length of the transaction when EIP-2718 encoded. This is the
    /// 1 byte type flag + the length of the RLP encoded transaction.
    pub fn eip2718_encoded_length(&self) -> usize {
        self.rlp_encoded_length() + 1
    }

    /// EIP-2718 encode the transaction with the given signature and the default
    /// type flag.
    pub fn eip2718_encode(&self, out: &mut dyn BufMut) {
        out.put_u8(self.tx_type() as u8);
        self.rlp_encode(out);
    }

    fn network_header(&self) -> Header {
        Header { list: false, payload_length: self.eip2718_encoded_length() }
    }

    /// Get the length of the transaction when network encoded. This is the
    /// EIP-2718 encoded length with an outer RLP header.
    pub fn network_encoded_length(&self) -> usize {
        self.network_header().length_with_payload()
    }

    /// Network encode the transaction with the given signature.
    pub fn network_encode(&self, out: &mut dyn BufMut) {
        self.network_header().encode(out);
        self.eip2718_encode(out);
    }

    /// Calculate the transaction hash.
    pub fn tx_hash(&self) -> TxHash {
        let mut buf = Vec::with_capacity(self.eip2718_encoded_length());
        self.eip2718_encode(&mut buf);
        keccak256(&buf)
    }

    /// Returns the signature for the optimism deposit transactions, which don't include a
    /// signature.
    pub fn signature() -> Signature {
        Signature::new(U256::ZERO, U256::ZERO, false)
    }
}

impl Typed2718 for TxDeposit {
    fn ty(&self) -> u8 {
        OpTxType::Deposit as u8
    }
}

impl Transaction for TxDeposit {
    fn chain_id(&self) -> Option<ChainId> {
        None
    }

    fn nonce(&self) -> u64 {
        0u64
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn gas_price(&self) -> Option<u128> {
        None
    }

    fn max_fee_per_gas(&self) -> u128 {
        0
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        None
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        None
    }

    fn priority_fee_or_price(&self) -> u128 {
        0
    }

    fn effective_gas_price(&self, _: Option<u64>) -> u128 {
        0
    }

    fn is_dynamic_fee(&self) -> bool {
        false
    }

    fn kind(&self) -> TxKind {
        self.to
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn input(&self) -> &Bytes {
        &self.input
    }

    fn access_list(&self) -> Option<&AccessList> {
        None
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        None
    }

    fn authorization_list(&self) -> Option<&[alloy_eips::eip7702::SignedAuthorization]> {
        None
    }

    fn is_create(&self) -> bool {
        self.to.is_create()
    }
}

impl Encodable for TxDeposit {
    fn encode(&self, out: &mut dyn BufMut) {
        Header { list: true, payload_length: self.rlp_encoded_fields_length() }.encode(out);
        self.rlp_encode_fields(out);
    }

    fn length(&self) -> usize {
        let payload_length = self.rlp_encoded_fields_length();
        Header { list: true, payload_length }.length() + payload_length
    }
}

impl Decodable for TxDeposit {
    fn decode(data: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::rlp_decode(data)
    }
}

impl Sealable for TxDeposit {
    fn hash_slow(&self) -> B256 {
        self.tx_hash()
    }
}

/// Deposit transactions don't have a signature, however, we include an empty signature in the
/// response for better compatibility.
///
/// This function can be used as `serialize_with` serde attribute for the [`TxDeposit`] and will
/// flatten [`TxDeposit::signature`] into response.
#[cfg(feature = "serde")]
pub fn serde_deposit_tx_rpc<T: serde::Serialize, S: serde::Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct SerdeHelper<'a, T> {
        #[serde(flatten)]
        value: &'a T,
        #[serde(flatten)]
        signature: Signature,
    }

    SerdeHelper { value, signature: TxDeposit::signature() }.serialize(serializer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;
    use alloy_rlp::BytesMut;

    #[test]
    fn test_deposit_transaction_trait() {
        let tx = TxDeposit {
            source_hash: B256::with_last_byte(42),
            from: Address::default(),
            to: TxKind::default(),
            mint: Some(100),
            value: U256::from(1000),
            gas_limit: 50000,
            is_system_transaction: true,
            input: Bytes::default(),
            eth_value: Some(100),
            eth_tx_value: Some(100),
        };

        assert_eq!(tx.source_hash(), Some(B256::with_last_byte(42)));
        assert_eq!(tx.mint(), Some(100));
        assert!(tx.is_system_transaction());
        assert!(tx.is_deposit());
    }

    #[test]
    fn test_deposit_transaction_without_mint() {
        let tx = TxDeposit {
            source_hash: B256::default(),
            from: Address::default(),
            to: TxKind::default(),
            mint: None,
            value: U256::default(),
            gas_limit: 50000,
            is_system_transaction: false,
            input: Bytes::default(),
            eth_value: None,
            eth_tx_value: None,
        };

        assert_eq!(tx.source_hash(), Some(B256::default()));
        assert_eq!(tx.mint(), None);
        assert!(!tx.is_system_transaction());
        assert!(tx.is_deposit());
    }

    #[test]
    fn test_deposit_transaction_to_contract() {
        let contract_address = Address::with_last_byte(0xFF);
        let tx = TxDeposit {
            source_hash: B256::default(),
            from: Address::default(),
            to: TxKind::Call(contract_address),
            mint: Some(200),
            value: U256::from(500),
            gas_limit: 100000,
            is_system_transaction: false,
            input: Bytes::from_static(&[1, 2, 3]),
            eth_value: Some(100),
            eth_tx_value: Some(100),
        };

        assert_eq!(tx.source_hash(), Some(B256::default()));
        assert_eq!(tx.mint(), Some(200));
        assert!(!tx.is_system_transaction());
        assert!(tx.is_deposit());
        assert_eq!(tx.kind(), TxKind::Call(contract_address));
    }

    #[test]
    fn test_rlp_roundtrip() {
        let bytes = Bytes::from_static(&hex!("7ef9015aa044bae9d41b8380d781187b426c6fe43df5fb2fb57bd4466ef6a701e1f01e015694deaddeaddeaddeaddeaddeaddeaddeaddead000194420000000000000000000000000000000000001580808408f0d18001b90104015d8eb900000000000000000000000000000000000000000000000000000000008057650000000000000000000000000000000000000000000000000000000063d96d10000000000000000000000000000000000000000000000000000000000009f35273d89754a1e0387b89520d989d3be9c37c1f32495a88faf1ea05c61121ab0d1900000000000000000000000000000000000000000000000000000000000000010000000000000000000000002d679b567db6187c0c8323fa982cfb88b74dbcc7000000000000000000000000000000000000000000000000000000000000083400000000000000000000000000000000000000000000000000000000000f4240"));
        let tx_a = TxDeposit::decode(&mut bytes[1..].as_ref()).unwrap();
        let mut buf_a = BytesMut::default();
        tx_a.encode(&mut buf_a);
        assert_eq!(&buf_a[..], &bytes[1..]);
    }

    #[test]
    fn test_encode_decode_fields() {
        let original = TxDeposit {
            source_hash: B256::default(),
            from: Address::default(),
            to: TxKind::default(),
            mint: Some(100),
            value: U256::default(),
            gas_limit: 50000,
            is_system_transaction: true,
            eth_value: Some(100),
            input: Bytes::default(),
            eth_tx_value: Some(100),
        };

        let mut buffer = BytesMut::new();
        original.rlp_encode_fields(&mut buffer);
        let decoded = TxDeposit::rlp_decode_fields(&mut &buffer[..]).expect("Failed to decode");

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_with_and_without_header() {
        let tx_deposit = TxDeposit {
            source_hash: B256::default(),
            from: Address::default(),
            to: TxKind::default(),
            mint: Some(100),
            value: U256::default(),
            gas_limit: 50000,
            is_system_transaction: true,
            eth_value: Some(100),
            input: Bytes::default(),
            eth_tx_value: Some(100),
        };

        let mut buffer_with_header = BytesMut::new();
        tx_deposit.encode(&mut buffer_with_header);

        let mut buffer_without_header = BytesMut::new();
        tx_deposit.rlp_encode_fields(&mut buffer_without_header);

        assert!(buffer_with_header.len() > buffer_without_header.len());
    }

    #[test]
    fn test_payload_length() {
        let tx_deposit = TxDeposit {
            source_hash: B256::default(),
            from: Address::default(),
            to: TxKind::default(),
            mint: Some(100),
            value: U256::default(),
            gas_limit: 50000,
            is_system_transaction: true,
            eth_value: Some(100),
            input: Bytes::default(),
            eth_tx_value: Some(100),
        };

        assert!(tx_deposit.size() > tx_deposit.rlp_encoded_fields_length());
    }

    #[test]
    fn test_encode_inner_with_and_without_header() {
        let tx_deposit = TxDeposit {
            source_hash: B256::default(),
            from: Address::default(),
            to: TxKind::default(),
            mint: Some(100),
            value: U256::default(),
            gas_limit: 50000,
            is_system_transaction: true,
            eth_value: Some(100),
            input: Bytes::default(),
            eth_tx_value: Some(100),
        };

        let mut buffer_with_header = BytesMut::new();
        tx_deposit.network_encode(&mut buffer_with_header);

        let mut buffer_without_header = BytesMut::new();
        tx_deposit.eip2718_encode(&mut buffer_without_header);

        assert!(buffer_with_header.len() > buffer_without_header.len());
    }

    #[test]
    fn test_payload_length_header() {
        let tx_deposit = TxDeposit {
            source_hash: B256::default(),
            from: Address::default(),
            to: TxKind::default(),
            mint: Some(100),
            value: U256::default(),
            gas_limit: 50000,
            is_system_transaction: true,
            eth_value: Some(100),
            input: Bytes::default(),
            eth_tx_value: Some(100),
        };

        let total_len = tx_deposit.network_encoded_length();
        let len_without_header = tx_deposit.eip2718_encoded_length();

        assert!(total_len > len_without_header);
    }
}

/// Bincode-compatible [`TxDeposit`] serde implementation.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(super) mod serde_bincode_compat {
    use alloc::borrow::Cow;
    use alloy_primitives::{Address, Bytes, TxKind, B256, U256};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Bincode-compatible [`super::TxDeposit`] serde implementation.
    ///
    /// Intended to use with the [`serde_with::serde_as`] macro in the following way:
    /// ```rust
    /// use op_alloy_consensus::{serde_bincode_compat, TxDeposit};
    /// use serde::{Deserialize, Serialize};
    /// use serde_with::serde_as;
    ///
    /// #[serde_as]
    /// #[derive(Serialize, Deserialize)]
    /// struct Data {
    ///     #[serde_as(as = "serde_bincode_compat::TxDeposit")]
    ///     transaction: TxDeposit,
    /// }
    /// ```
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TxDeposit<'a> {
        source_hash: B256,
        from: Address,
        #[serde(default)]
        to: TxKind,
        #[serde(default)]
        mint: Option<u128>,
        value: U256,
        gas_limit: u64,
        is_system_transaction: bool,
        input: Cow<'a, Bytes>,
        #[serde(default)]
        eth_value: Option<u128>,
        eth_tx_value: Option<u128>,
    }

    impl<'a> From<&'a super::TxDeposit> for TxDeposit<'a> {
        fn from(value: &'a super::TxDeposit) -> Self {
            Self {
                source_hash: value.source_hash,
                from: value.from,
                to: value.to,
                mint: value.mint,
                value: value.value,
                gas_limit: value.gas_limit,
                is_system_transaction: value.is_system_transaction,
                eth_value: value.eth_value,
                input: Cow::Borrowed(&value.input),
                eth_tx_value: value.eth_tx_value,
            }
        }
    }

    impl<'a> From<TxDeposit<'a>> for super::TxDeposit {
        fn from(value: TxDeposit<'a>) -> Self {
            Self {
                source_hash: value.source_hash,
                from: value.from,
                to: value.to,
                mint: value.mint,
                value: value.value,
                gas_limit: value.gas_limit,
                is_system_transaction: value.is_system_transaction,
                eth_value: value.eth_value,
                input: value.input.into_owned(),
                eth_tx_value: value.eth_tx_value,
            }
        }
    }

    impl SerializeAs<super::TxDeposit> for TxDeposit<'_> {
        fn serialize_as<S>(source: &super::TxDeposit, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            TxDeposit::from(source).serialize(serializer)
        }
    }

    impl<'de> DeserializeAs<'de, super::TxDeposit> for TxDeposit<'de> {
        fn deserialize_as<D>(deserializer: D) -> Result<super::TxDeposit, D::Error>
        where
            D: Deserializer<'de>,
        {
            TxDeposit::deserialize(deserializer).map(Into::into)
        }
    }

    #[cfg(test)]
    mod tests {
        use arbitrary::Arbitrary;
        use rand::Rng;
        use serde::{Deserialize, Serialize};
        use serde_with::serde_as;

        use super::super::{serde_bincode_compat, TxDeposit};

        #[test]
        fn test_tx_deposit_bincode_roundtrip() {
            #[serde_as]
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Data {
                #[serde_as(as = "serde_bincode_compat::TxDeposit")]
                transaction: TxDeposit,
            }

            let mut bytes = [0u8; 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());
            let data = Data {
                transaction: TxDeposit::arbitrary(&mut arbitrary::Unstructured::new(&bytes))
                    .unwrap(),
            };

            let encoded = bincode::serialize(&data).unwrap();
            let decoded: Data = bincode::deserialize(&encoded).unwrap();
            assert_eq!(decoded, data);
        }
    }
}
