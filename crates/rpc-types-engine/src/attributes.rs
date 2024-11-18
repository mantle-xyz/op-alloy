//! Optimism-specific payload attributes.

use alloc::vec::Vec;
use alloy_primitives::{Bytes, B64};
use alloy_rpc_types_engine::PayloadAttributes;
use op_alloy_protocol::L2BlockInfo;

/// Optimism Payload Attributes
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OpPayloadAttributes {
    /// The payload attributes
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub payload_attributes: PayloadAttributes,
    /// Transactions is a field for rollups: the transactions list is forced into the block
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub transactions: Option<Vec<Bytes>>,
    /// If true, the no transactions are taken out of the tx-pool, only transactions from the above
    /// Transactions list will be included.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub no_tx_pool: Option<bool>,
    /// If set, this sets the exact gas limit the block produced with.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")
    )]
    pub gas_limit: Option<u64>,
    /// If set, this sets the EIP-1559 parameters for the block.
    ///
    /// Prior to Holocene activation, this field should always be [None].
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub base_fee: Option<u128>,
}

/// Optimism Payload Attributes with parent block reference.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpAttributesWithParent {
    /// The payload attributes.
    pub attributes: OpPayloadAttributes,
    /// The parent block reference.
    pub parent: L2BlockInfo,
}

impl OpAttributesWithParent {
    /// Create a new [OpAttributesWithParent] instance.
    pub const fn new(
        attributes: OpPayloadAttributes,
        parent: L2BlockInfo,
    ) -> Self {
        Self { attributes, parent }
    }

    /// Returns the payload attributes.
    pub const fn attributes(&self) -> &OpPayloadAttributes {
        &self.attributes
    }

    /// Returns the parent block reference.
    pub const fn parent(&self) -> &L2BlockInfo {
        &self.parent
    }

}

#[cfg(all(test, feature = "serde"))]
mod test {
    use super::*;
    use alloy_primitives::{b64, Address, B256};
    use alloy_rpc_types_engine::PayloadAttributes;

    #[test]
    fn test_serde_roundtrip_attributes_pre_holocene() {
        let attributes = OpPayloadAttributes {
            payload_attributes: PayloadAttributes {
                timestamp: 0x1337,
                prev_randao: B256::ZERO,
                suggested_fee_recipient: Address::ZERO,
                withdrawals: Default::default(),
                parent_beacon_block_root: Some(B256::ZERO),
            },
            transactions: Some(vec![b"hello".to_vec().into()]),
            no_tx_pool: Some(true),
            gas_limit: Some(42),
            base_fee: Some(100_000),
        };

        let ser = serde_json::to_string(&attributes).unwrap();
        let de: OpPayloadAttributes = serde_json::from_str(&ser).unwrap();

        assert_eq!(attributes, de);
    }

}
