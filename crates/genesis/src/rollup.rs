//! Rollup Config Types

use alloy_primitives::{address, b256, uint, Address};

use alloy_eips::eip1898::BlockNumHash;

use crate::{
     ChainGenesis, SystemConfig,
};

/// The max rlp bytes per channel for the Bedrock hardfork.
pub const MAX_RLP_BYTES_PER_CHANNEL_BEDROCK: u64 = 10_000_000;

/// Returns the rollup config for the given chain ID.
pub fn rollup_config_from_chain_id(chain_id: u64) -> Result<RollupConfig, &'static str> {
    chain_id.try_into()
}

impl TryFrom<u64> for RollupConfig {
    type Error = &'static str;

    fn try_from(chain_id: u64) -> Result<Self, &'static str> {
        match chain_id {
            5000 => Ok(MANTLE_MAINNET_CONFIG),
            5003 => Ok(MANTLE_SEPOLIA_CONFIG),
            _ => Err("Unknown chain ID"),
        }
    }
}

/// The Rollup configuration.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RollupConfig {
    /// The genesis state of the rollup.
    pub genesis: ChainGenesis,
    /// The block time of the L2, in seconds.
    pub block_time: u64,
    /// Sequencer batches may not be more than MaxSequencerDrift seconds after
    /// the L1 timestamp of the sequencing window end.
    ///
    /// Note: When L1 has many 1 second consecutive blocks, and L2 grows at fixed 2 seconds,
    /// the L2 time may still grow beyond this difference.
    ///
    /// Note: After the Fjord hardfork, this value becomes a constant of `1800`.
    pub max_sequencer_drift: u64,
    /// The sequencer window size.
    pub seq_window_size: u64,
    /// Number of L1 blocks between when a channel can be opened and when it can be closed.
    pub channel_timeout: u64,
    /// The L1 chain ID
    pub l1_chain_id: u64,
    /// The L2 chain ID
    pub l2_chain_id: u64,
    /// `regolith_time` sets the activation time of the Regolith network-upgrade:
    /// a pre-mainnet Bedrock change that addresses findings of the Sherlock contest related to
    /// deposit attributes. "Regolith" is the loose deposited rock that sits on top of Bedrock.
    /// Active if regolith_time != None && L2 block timestamp >= Some(regolith_time), inactive
    /// otherwise.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub regolith_time: Option<u64>,
    /// BaseFeeTime sets the activation time of the BaseFee network-upgrade:
    /// Active if BaseFeeTime != nil && L2 block tmestamp >= *BaseFeeTime, inactive otherwise.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub base_fee_time: Option<u64>,
    /// `batch_inbox_address` is the L1 address that batches are sent to.
    pub batch_inbox_address: Address,
    /// `deposit_contract_address` is the L1 address that deposits are sent to.
    pub deposit_contract_address: Address,
    /// `l1_system_config_address` is the L1 address that the system config is stored at.
    pub l1_system_config_address: Address,
    /// `mantle_da_switch` is a switch that weather use mantle da.
    pub mantle_da_switch:bool,
    /// `datalayr_service_manager_addr` is the mantle da manager address that the data availability contract.
    pub datalayr_service_manager_addr: Address,
    /// `shanghai_time` defined here just for mantle revm to use. no config in mantle rollup config file, actually.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub shanghai_time: Option<u64>,

}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for RollupConfig {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            genesis: ChainGenesis::arbitrary(u)?,
            block_time: u.arbitrary()?,
            max_sequencer_drift: u.arbitrary()?,
            seq_window_size: u.arbitrary()?,
            channel_timeout: u.arbitrary()?,
            l1_chain_id: u.arbitrary()?,
            l2_chain_id: u.arbitrary()?,
            regolith_time: Option::<u64>::arbitrary(u)?,
            base_fee_time: Option::<u64>::arbitrary(u)?,
            batch_inbox_address: Address::arbitrary(u)?,
            deposit_contract_address: Address::arbitrary(u)?,
            l1_system_config_address: Address::arbitrary(u)?,
            mantle_da_switch: u.arbitrary()?,
            datalayr_service_manager_addr: Address::default(),
            shanghai_time: Option::<u64>::arbitrary(u)?,
        })
    }
}

// Need to manually implement Default because [`BaseFeeParams`] has no Default impl.
impl Default for RollupConfig {
    fn default() -> Self {
        Self {
            genesis: ChainGenesis::default(),
            block_time: 0,
            max_sequencer_drift: 0,
            seq_window_size: 0,
            channel_timeout: 0,
            l1_chain_id: 0,
            l2_chain_id: 0,
            regolith_time: None,
            base_fee_time: None,
            batch_inbox_address: Address::ZERO,
            deposit_contract_address: Address::ZERO,
            l1_system_config_address: Address::ZERO,
            mantle_da_switch: false,
            datalayr_service_manager_addr: Address::ZERO,
            shanghai_time: None,
        }
    }
}

impl RollupConfig {
    /// Returns true if Regolith is active at the given timestamp.
    pub fn is_regolith_active(&self, timestamp: u64) -> bool {
        self.regolith_time.map_or(false, |t| timestamp >= t)
    }

    pub fn is_shanghai_active(&self, timestamp: u64) -> bool {
        self.shanghai_time.map_or(false, |t| timestamp >= t)
    }


    /// Returns the [RollupConfig] for the given L2 chain ID.
    pub const fn from_l2_chain_id(l2_chain_id: u64) -> Option<Self> {
        match l2_chain_id {
            5000 => Some(MANTLE_MAINNET_CONFIG),
            5003 => Some(MANTLE_SEPOLIA_CONFIG),
            _ => None,
        }
    }

    /// Returns the max sequencer drift for the given timestamp.
    pub fn max_sequencer_drift(&self, timestamp: u64) -> u64 {
        self.max_sequencer_drift
    }

    /// Returns the max rlp bytes per channel for the given timestamp.
    pub fn max_rlp_bytes_per_channel(&self, timestamp: u64) -> u64 {
        MAX_RLP_BYTES_PER_CHANNEL_BEDROCK
    }

    /// Returns the channel timeout for the given timestamp.
    pub fn channel_timeout(&self, timestamp: u64) -> u64 {
        self.channel_timeout
    }
}

/// The [RollupConfig] for MANTLE Mainnet.
pub const MANTLE_MAINNET_CONFIG: RollupConfig = RollupConfig {
    genesis: ChainGenesis {
        l1: BlockNumHash {
            hash: b256!("614050145039f11a778f1bd3c85ce2c1f3989492dbc544911fab9a7247e81ca4"),
            number: 19_437_305_u64,
        },
        l2: BlockNumHash {
            hash: b256!("f70a2270b05820a2b335e70ab9ce91e42e15f50d82db73d9c63085711b312fc8"),
            number: 61_171_946_u64,
        },
        l2_time: 1_710_468_791_u64,
        system_config: Some(SystemConfig {
            batcher_address: address!("2f40d796917ffb642bd2e2bdd2c762a5e40fd749"),
            overhead: uint!(0xbc_U256),
            scalar: uint!(0x2710_U256),
            gas_limit: 200_000_000_000_u64,
            base_fee: uint!(0x1312d00_U256),
        }),
    },
    block_time: 2_u64,
    max_sequencer_drift: 600_u64,
    seq_window_size: 3600_u64,
    channel_timeout: 300_u64,
    l1_chain_id: 1_u64,
    l2_chain_id: 5_000_u64,
    regolith_time: Some(0_u64),
    base_fee_time: None,
    batch_inbox_address: address!("ff00000000000000000000000000000000000000"),
    deposit_contract_address: address!("c54cb22944f2be476e02decfcd7e3e7d3e15a8fb"),
    l1_system_config_address: address!("427ea0710fa5252057f0d88274f7aeb308386caf"),
    mantle_da_switch: true,
    datalayr_service_manager_addr: address!("5BD63a7ECc13b955C4F57e3F12A64c10263C14c1"),
    shanghai_time: Some(0_u64),
};

/// The [RollupConfig] for MANTLE Sepolia.
pub const MANTLE_SEPOLIA_CONFIG: RollupConfig = RollupConfig {
    genesis: ChainGenesis {
        l1: BlockNumHash {
            hash: b256!("041dea101b3d09fee3dc566c9de820eca07d9d0e951853257c64c79fe4b90f25"),
            number: 4858225,
        },
        l2: BlockNumHash {
            hash: b256!("227de3c9c89eb8b8f88a26a06abe125c0d9c7a95a8213f7c83d098e7391bbde6"),
            number: 325709,
        },
        l2_time: 1702194288,
        system_config: Some(SystemConfig {
            batcher_address: address!("5fb5139834df283b6a4bd7267952f3ea21a573f4"),
            overhead: uint!(0x834_U256),
            scalar: uint!(0xf4240_U256),
            gas_limit: 1_125_899_906_842_624,
            base_fee: uint!(0x3b9aca00_U256),
        }),
    },
    block_time: 2,
    max_sequencer_drift: 600,
    seq_window_size: 3600,
    channel_timeout: 300,
    l1_chain_id: 11155111,
    l2_chain_id: 5003,
    regolith_time: Some(0),
    base_fee_time: None,
    batch_inbox_address: address!("ff00000000000000000000000000000000000000"),
    deposit_contract_address: address!("b3db4bd5bc225930ed674494f9a4f6a11b8efbc8"),
    l1_system_config_address: address!("04b34526c91424e955d13c7226bc4385e57e6706"),
    mantle_da_switch: true,
    datalayr_service_manager_addr: address!("d7f17171896461A6EB74f95DF3f9b0D966A8a907"),
    shanghai_time: Some(0),
};


#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "serde")]
    use alloy_primitives::U256;
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn test_arbitrary_rollup_config() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());
        RollupConfig::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    fn test_regolith_active() {
        let mut config = RollupConfig::default();
        assert!(!config.is_regolith_active(0));
        config.regolith_time = Some(10);
        assert!(config.is_regolith_active(10));
        assert!(!config.is_regolith_active(9));
    }









    #[test]
    #[cfg(feature = "serde")]
    fn test_deserialize_reference_rollup_config() {
        // Reference serialized rollup config from the `op-node`.
        let ser_cfg = r#"
{
  "genesis": {
    "l1": {
      "hash": "0x481724ee99b1f4cb71d826e2ec5a37265f460e9b112315665c977f4050b0af54",
      "number": 10
    },
    "l2": {
      "hash": "0x88aedfbf7dea6bfa2c4ff315784ad1a7f145d8f650969359c003bbed68c87631",
      "number": 0
    },
    "l2_time": 1725557164,
    "system_config": {
      "batcherAddr": "0xc81f87a644b41e49b3221f41251f15c6cb00ce03",
      "overhead": "0x0000000000000000000000000000000000000000000000000000000000000000",
      "scalar": "0x00000000000000000000000000000000000000000000000000000000000f4240",
      "gasLimit": 30000000
    }
  },
  "block_time": 2,
  "max_sequencer_drift": 600,
  "seq_window_size": 3600,
  "channel_timeout": 300,
  "l1_chain_id": 3151908,
  "l2_chain_id": 1337,
  "regolith_time": 0,
  "batch_inbox_address": "0xff00000000000000000000000000000000042069",
  "deposit_contract_address": "0x08073dc48dde578137b8af042bcbc1c2491f1eb2",
  "l1_system_config_address": "0x94ee52a9d8edd72a85dea7fae3ba6d75e4bf1710",
  "mantle_da_switch": true,
  "datalayr_service_manager_addr": "0x5BD63a7ECc13b955C4F57e3F12A64c10263C14c1"
}
        "#;
        let config: RollupConfig = serde_json::from_str(ser_cfg).unwrap();

        // Validate standard fields.
        assert_eq!(
            config.genesis,
            ChainGenesis {
                l1: BlockNumHash {
                    hash: b256!("481724ee99b1f4cb71d826e2ec5a37265f460e9b112315665c977f4050b0af54"),
                    number: 10
                },
                l2: BlockNumHash {
                    hash: b256!("88aedfbf7dea6bfa2c4ff315784ad1a7f145d8f650969359c003bbed68c87631"),
                    number: 0
                },
                l2_time: 1725557164,
                system_config: Some(SystemConfig {
                    batcher_address: address!("c81f87a644b41e49b3221f41251f15c6cb00ce03"),
                    overhead: U256::ZERO,
                    scalar: U256::from(0xf4240),
                    gas_limit: 30_000_000,
                    base_fee: U256::ZERO,
                })
            }
        );
        assert_eq!(config.block_time, 2);
        assert_eq!(config.max_sequencer_drift, 600);
        assert_eq!(config.seq_window_size, 3600);
        assert_eq!(config.channel_timeout, 300);
        assert_eq!(config.l1_chain_id, 3151908);
        assert_eq!(config.l2_chain_id, 1337);
        assert_eq!(config.regolith_time, Some(0));
        assert_eq!(
            config.batch_inbox_address,
            address!("ff00000000000000000000000000000000042069")
        );
        assert_eq!(
            config.deposit_contract_address,
            address!("08073dc48dde578137b8af042bcbc1c2491f1eb2")
        );
        assert_eq!(
            config.l1_system_config_address,
            address!("94ee52a9d8edd72a85dea7fae3ba6d75e4bf1710")
        );
        assert_eq!(config.mantle_da_switch, true);
        assert_eq!(config.datalayr_service_manager_addr,
            address!("5BD63a7ECc13b955C4F57e3F12A64c10263C14c1")
        );

    }
}
