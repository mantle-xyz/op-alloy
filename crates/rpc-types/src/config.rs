#![allow(missing_docs)]
//! OP rollup config types.

use alloy_eips::BlockNumHash;
use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

// https://github.com/ethereum-optimism/optimism/blob/c7ad0ebae5dca3bf8aa6f219367a95c15a15ae41/op-service/eth/types.go#L371
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfig {
    pub batcher_addr: Address,
    pub overhead: B256,
    pub scalar: B256,
    pub gas_limit: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Genesis {
    pub l1: BlockNumHash,
    pub l2: BlockNumHash,
    pub l2_time: u64,
    pub system_config: SystemConfig,
}

// <https://github.com/ethereum-optimism/optimism/blob/77c91d09eaa44d2c53bec60eb89c5c55737bc325/op-node/rollup/types.go#L66>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollupConfig {
    pub genesis: Genesis,
    pub block_time: u64,
    pub max_sequencer_drift: u64,
    pub seq_window_size: u64,

    #[serde(rename = "channel_timeout")]
    pub channel_timeout_bedrock: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l1_chain_id: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l2_chain_id: Option<u128>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regolith_time: Option<u64>,
    pub batch_inbox_address: Address,
    pub deposit_contract_address: Address,
    pub l1_system_config_address: Address,
    pub mantle_da_switch: bool,
    pub datalayr_service_manager_addr: Address,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollup_config() {
        let s = r#"{
      "genesis": {
        "l1": {
          "hash": "0x041dea101b3d09fee3dc566c9de820eca07d9d0e951853257c64c79fe4b90f25",
          "number": 4858225
        },
        "l2": {
          "hash": "0x227de3c9c89eb8b8f88a26a06abe125c0d9c7a95a8213f7c83d098e7391bbde6",
          "number": 325709
        },
        "l2_time": 1702194288,
        "system_config": {
          "batcherAddr": "0x5fb5139834df283b6a4bd7267952f3ea21a573f4",
          "overhead": "0x0000000000000000000000000000000000000000000000000000000000000834",
          "scalar": "0x00000000000000000000000000000000000000000000000000000000000f4240",
          "baseFee": 1000000000,
          "gasLimit": 1125899906842624
        }
      },
      "block_time": 2,
      "max_sequencer_drift": 600,
      "seq_window_size": 3600,
      "channel_timeout": 300,
      "l1_chain_id": 11155111,
      "l2_chain_id": 5003,
      "regolith_time": 0,
      "batch_inbox_address": "0xff00000000000000000000000000000000000000",
      "deposit_contract_address": "0xb3db4bd5bc225930ed674494f9a4f6a11b8efbc8",
      "l1_system_config_address": "0x04b34526c91424e955d13c7226bc4385e57e6706",
      "mantle_da_switch": true,
      "datalayr_service_manager_addr": "0xd7f17171896461A6EB74f95DF3f9b0D966A8a907"
    }"#;

        let deserialize = serde_json::from_str::<RollupConfig>(s).unwrap();

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(s).unwrap(),
            serde_json::to_value(&deserialize).unwrap()
        );
    }
}
