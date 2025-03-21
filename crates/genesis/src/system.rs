//! System Config Type

use crate::RollupConfig;
use alloy_consensus::{Eip658Value, Receipt};
use alloy_primitives::{address, b256, Address, Log, B256, U256, U64};
use alloy_sol_types::{sol, SolType};

/// `keccak256("ConfigUpdate(uint256,uint8,bytes)")`
pub const CONFIG_UPDATE_TOPIC: B256 =
    b256!("1d2b0bda21d56b8bd12d4f94ebacffdfb35f5e226f84b461103bb8beab6353be");

/// The initial version of the system config event log.
pub const CONFIG_UPDATE_EVENT_VERSION_0: B256 = B256::ZERO;

/// System configuration.
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SystemConfig {
    /// Batcher address
    #[cfg_attr(feature = "serde", serde(rename = "batcherAddr"))]
    pub batcher_address: Address,
    /// Fee overhead value
    pub overhead: U256,
    /// Fee scalar value
    pub scalar: U256,
    /// Gas limit value
    pub gas_limit: u64,
    /// BaseFee identifies the L2 block base fee
    pub base_fee: U256,
}

/// Represents type of update to the system config.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(u64)]
pub enum SystemConfigUpdateType {
    /// Batcher update type
    Batcher = 0,
    /// Gas config update type
    GasConfig = 1,
    /// Gas limit update type
    GasLimit = 2,
    /// Unsafe block signer update type
    UnsafeBlockSigner = 3,
    /// EIP-1559 parameters update type
    BaseFee = 4,
}

impl TryFrom<u64> for SystemConfigUpdateType {
    type Error = SystemConfigUpdateError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Batcher),
            1 => Ok(Self::GasConfig),
            2 => Ok(Self::GasLimit),
            3 => Ok(Self::UnsafeBlockSigner),
            4 => Ok(Self::BaseFee),
            _ => Err(SystemConfigUpdateError::LogProcessing(
                LogProcessingError::InvalidSystemConfigUpdateType(value),
            )),
        }
    }
}

impl SystemConfig {
    /// Filters all L1 receipts to find config updates and applies the config updates.
    pub fn update_with_receipts(
        &mut self,
        receipts: &[Receipt],
        l1_system_config_address: Address,
    ) -> Result<(), SystemConfigUpdateError> {
        for receipt in receipts {
            if Eip658Value::Eip658(false) == receipt.status {
                continue;
            }

            receipt.logs.iter().try_for_each(|log| {
                let topics = log.topics();
                if log.address == l1_system_config_address
                    && !topics.is_empty()
                    && topics[0] == CONFIG_UPDATE_TOPIC
                {
                    // Safety: Error is bubbled up by the trailing `?`
                    self.process_config_update_log(log)?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    /// Decodes an EVM log entry emitted by the system config contract and applies it as a
    /// [SystemConfig] change.
    ///
    /// Parse log data for:
    ///
    /// ```text
    /// event ConfigUpdate(
    ///    uint256 indexed version,
    ///    UpdateType indexed updateType,
    ///    bytes data
    /// );
    /// ```
    fn process_config_update_log(
        &mut self,
        log: &Log,
    ) -> Result<SystemConfigUpdateType, SystemConfigUpdateError> {
        // Validate the log
        if log.topics().len() < 3 {
            return Err(SystemConfigUpdateError::LogProcessing(
                LogProcessingError::InvalidTopicLen(log.topics().len()),
            ));
        }
        if log.topics()[0] != CONFIG_UPDATE_TOPIC {
            return Err(SystemConfigUpdateError::LogProcessing(LogProcessingError::InvalidTopic));
        }

        // Parse the config update log
        let version = log.topics()[1];
        if version != CONFIG_UPDATE_EVENT_VERSION_0 {
            return Err(SystemConfigUpdateError::LogProcessing(
                LogProcessingError::UnsupportedVersion(version),
            ));
        }
        let Ok(topic_bytes) = <&[u8; 8]>::try_from(&log.topics()[2].as_slice()[24..]) else {
            return Err(SystemConfigUpdateError::LogProcessing(
                LogProcessingError::UpdateTypeDecodingError,
            ));
        };
        let update_type = u64::from_be_bytes(*topic_bytes);
        let log_data = log.data.data.as_ref();

        // Apply the update
        match update_type.try_into()? {
            SystemConfigUpdateType::Batcher => {
                self.update_batcher_address(log_data).map_err(SystemConfigUpdateError::Batcher)
            }
            SystemConfigUpdateType::GasConfig => {
                self.update_gas_config(log_data).map_err(SystemConfigUpdateError::GasConfig)
            }
            SystemConfigUpdateType::GasLimit => {
                self.update_gas_limit(log_data).map_err(SystemConfigUpdateError::GasLimit)
            }
            SystemConfigUpdateType::BaseFee => {
                self.update_base_fee_config(log_data).map_err(SystemConfigUpdateError::BaseFee)
            }
            // Ignored in derivation
            SystemConfigUpdateType::UnsafeBlockSigner => {
                Ok(SystemConfigUpdateType::UnsafeBlockSigner)
            }
        }
    }

    /// Updates the batcher address in the [SystemConfig] given the log data.
    fn update_batcher_address(
        &mut self,
        log_data: &[u8],
    ) -> Result<SystemConfigUpdateType, BatcherUpdateError> {
        if log_data.len() != 96 {
            return Err(BatcherUpdateError::InvalidDataLen(log_data.len()));
        }

        let Ok(pointer) = <sol!(uint64)>::abi_decode(&log_data[0..32], true) else {
            return Err(BatcherUpdateError::PointerDecodingError);
        };
        if pointer != 32 {
            return Err(BatcherUpdateError::InvalidDataPointer(pointer));
        }
        let Ok(length) = <sol!(uint64)>::abi_decode(&log_data[32..64], true) else {
            return Err(BatcherUpdateError::LengthDecodingError);
        };
        if length != 32 {
            return Err(BatcherUpdateError::InvalidDataLength(length));
        }

        let Ok(batcher_address) = <sol!(address)>::abi_decode(&log_data[64..], true) else {
            return Err(BatcherUpdateError::BatcherAddressDecodingError);
        };
        self.batcher_address = batcher_address;
        Ok(SystemConfigUpdateType::Batcher)
    }

    fn update_base_fee_config(
        &mut self,
        log_data: &[u8],
    ) -> Result<SystemConfigUpdateType, BaseFeeUpdateError> {
        if log_data.len() != 96 {
            return Err(BaseFeeUpdateError::InvalidDataLen(log_data.len()));
        }

        let Ok(pointer) = <sol!(uint64)>::abi_decode(&log_data[0..32], true) else {
            return Err(BaseFeeUpdateError::PointerDecodingError);
        };
        if pointer != 32 {
            return Err(BaseFeeUpdateError::InvalidDataPointer(pointer));
        }
        let Ok(length) = <sol!(uint64)>::abi_decode(&log_data[32..64], true) else {
            return Err(BaseFeeUpdateError::LengthDecodingError);
        };
        if length != 32 {
            return Err(BaseFeeUpdateError::InvalidDataLength(length));
        }

        let Ok(base_fee) = <sol!(uint256)>::abi_decode(&log_data[64..], true) else {
            return Err(BaseFeeUpdateError::BaseFeeDecodingError);
        };
        self.base_fee = base_fee;
        Ok(SystemConfigUpdateType::BaseFee)
    }

    /// Updates the [SystemConfig] gas config - both the overhead and scalar values
    /// given the log data and rollup config.
    fn update_gas_config(
        &mut self,
        log_data: &[u8],
    ) -> Result<SystemConfigUpdateType, GasConfigUpdateError> {
        if log_data.len() != 128 {
            return Err(GasConfigUpdateError::InvalidDataLen(log_data.len()));
        }

        let Ok(pointer) = <sol!(uint64)>::abi_decode(&log_data[0..32], true) else {
            return Err(GasConfigUpdateError::PointerDecodingError);
        };
        if pointer != 32 {
            return Err(GasConfigUpdateError::InvalidDataPointer(pointer));
        }
        let Ok(length) = <sol!(uint64)>::abi_decode(&log_data[32..64], true) else {
            return Err(GasConfigUpdateError::LengthDecodingError);
        };
        if length != 64 {
            return Err(GasConfigUpdateError::InvalidDataLength(length));
        }

        let Ok(overhead) = <sol!(uint256)>::abi_decode(&log_data[64..96], true) else {
            return Err(GasConfigUpdateError::OverheadDecodingError);
        };
        let Ok(scalar) = <sol!(uint256)>::abi_decode(&log_data[96..], true) else {
            return Err(GasConfigUpdateError::ScalarDecodingError);
        };

        // Retain the scalar data in encoded form.
        self.scalar = scalar;

        // If ecotone is active, set the overhead to zero, otherwise set to the decoded value.
        self.overhead = overhead;

        Ok(SystemConfigUpdateType::GasConfig)
    }

    /// Updates the gas limit of the [SystemConfig] given the log data.
    fn update_gas_limit(
        &mut self,
        log_data: &[u8],
    ) -> Result<SystemConfigUpdateType, GasLimitUpdateError> {
        if log_data.len() != 96 {
            return Err(GasLimitUpdateError::InvalidDataLen(log_data.len()));
        }

        let Ok(pointer) = <sol!(uint64)>::abi_decode(&log_data[0..32], true) else {
            return Err(GasLimitUpdateError::PointerDecodingError);
        };
        if pointer != 32 {
            return Err(GasLimitUpdateError::InvalidDataPointer(pointer));
        }
        let Ok(length) = <sol!(uint64)>::abi_decode(&log_data[32..64], true) else {
            return Err(GasLimitUpdateError::LengthDecodingError);
        };
        if length != 32 {
            return Err(GasLimitUpdateError::InvalidDataLength(length));
        }

        let Ok(gas_limit) = <sol!(uint256)>::abi_decode(&log_data[64..], true) else {
            return Err(GasLimitUpdateError::GasLimitDecodingError);
        };
        self.gas_limit = U64::from(gas_limit).saturating_to::<u64>();
        Ok(SystemConfigUpdateType::GasLimit)
    }
}

/// An error for processing the [SystemConfig] update log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SystemConfigUpdateError {
    /// An error occurred while processing the update log.
    LogProcessing(LogProcessingError),
    /// A batcher update error.
    Batcher(BatcherUpdateError),
    /// A gas config update error.
    GasConfig(GasConfigUpdateError),
    /// A gas limit update error.
    GasLimit(GasLimitUpdateError),
    /// An EIP-1559 parameter update error.
    BaseFee(BaseFeeUpdateError),
}

impl core::fmt::Display for SystemConfigUpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LogProcessing(err) => write!(f, "Log processing error: {}", err),
            Self::Batcher(err) => write!(f, "Batcher update error: {}", err),
            Self::GasConfig(err) => write!(f, "Gas config update error: {}", err),
            Self::GasLimit(err) => write!(f, "Gas limit update error: {}", err),
            Self::BaseFee(err) => write!(f, "EIP-1559 parameter update error: {}", err),
        }
    }
}

/// An error occurred while processing the update log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LogProcessingError {
    /// Received an incorrect number of log topics.
    InvalidTopicLen(usize),
    /// The log topic is invalid.
    InvalidTopic,
    /// The config update log version is unsupported.
    UnsupportedVersion(B256),
    /// Failed to decode the update type from the config update log.
    UpdateTypeDecodingError,
    /// An invalid system config update type.
    InvalidSystemConfigUpdateType(u64),
}

impl core::fmt::Display for LogProcessingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidTopicLen(len) => {
                write!(f, "Invalid config update log: invalid topic length: {}", len)
            }
            Self::InvalidTopic => write!(f, "Invalid config update log: invalid topic"),
            Self::UnsupportedVersion(version) => {
                write!(f, "Invalid config update log: unsupported version: {}", version)
            }
            Self::UpdateTypeDecodingError => {
                write!(f, "Failed to decode config update log: update type")
            }
            Self::InvalidSystemConfigUpdateType(value) => {
                write!(f, "Invalid system config update type: {}", value)
            }
        }
    }
}

/// An error for updating the batcher address on the [SystemConfig].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BatcherUpdateError {
    /// Invalid data length.
    InvalidDataLen(usize),
    /// Failed to decode the data pointer argument from the batcher update log.
    PointerDecodingError,
    /// The data pointer is invalid.
    InvalidDataPointer(u64),
    /// Failed to decode the data length argument from the batcher update log.
    LengthDecodingError,
    /// The data length is invalid.
    InvalidDataLength(u64),
    /// Failed to decode the batcher address argument from the batcher update log.
    BatcherAddressDecodingError,
}

impl core::fmt::Display for BatcherUpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDataLen(len) => {
                write!(f, "Invalid config update log: invalid data length: {}", len)
            }
            Self::PointerDecodingError => {
                write!(f, "Failed to decode batcher update log: data pointer")
            }
            Self::InvalidDataPointer(pointer) => {
                write!(f, "Invalid config update log: invalid data pointer: {}", pointer)
            }
            Self::LengthDecodingError => {
                write!(f, "Failed to decode batcher update log: data length")
            }
            Self::InvalidDataLength(length) => {
                write!(f, "Invalid config update log: invalid data length: {}", length)
            }
            Self::BatcherAddressDecodingError => {
                write!(f, "Failed to decode batcher update log: batcher address")
            }
        }
    }
}

/// An error for updating the gas config on the [SystemConfig].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GasConfigUpdateError {
    /// Invalid data length.
    InvalidDataLen(usize),
    /// Failed to decode the data pointer argument from the gas config update log.
    PointerDecodingError,
    /// The data pointer is invalid.
    InvalidDataPointer(u64),
    /// Failed to decode the data length argument from the gas config update log.
    LengthDecodingError,
    /// The data length is invalid.
    InvalidDataLength(u64),
    /// Failed to decode the overhead argument from the gas config update log.
    OverheadDecodingError,
    /// Failed to decode the scalar argument from the gas config update log.
    ScalarDecodingError,
}

impl core::fmt::Display for GasConfigUpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDataLen(len) => {
                write!(f, "Invalid config update log: invalid data length: {}", len)
            }
            Self::PointerDecodingError => {
                write!(f, "Failed to decode gas config update log: data pointer")
            }
            Self::InvalidDataPointer(pointer) => {
                write!(f, "Invalid config update log: invalid data pointer: {}", pointer)
            }
            Self::LengthDecodingError => {
                write!(f, "Failed to decode gas config update log: data length")
            }
            Self::InvalidDataLength(length) => {
                write!(f, "Invalid config update log: invalid data length: {}", length)
            }
            Self::OverheadDecodingError => {
                write!(f, "Failed to decode gas config update log: overhead")
            }
            Self::ScalarDecodingError => {
                write!(f, "Failed to decode gas config update log: scalar")
            }
        }
    }
}

/// An error for updating the gas limit on the [SystemConfig].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GasLimitUpdateError {
    /// Invalid data length.
    InvalidDataLen(usize),
    /// Failed to decode the data pointer argument from the gas limit update log.
    PointerDecodingError,
    /// The data pointer is invalid.
    InvalidDataPointer(u64),
    /// Failed to decode the data length argument from the gas limit update log.
    LengthDecodingError,
    /// The data length is invalid.
    InvalidDataLength(u64),
    /// Failed to decode the gas limit argument from the gas limit update log.
    GasLimitDecodingError,
}

impl core::fmt::Display for GasLimitUpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDataLen(len) => {
                write!(f, "Invalid config update log: invalid data length: {}", len)
            }
            Self::PointerDecodingError => {
                write!(f, "Failed to decode gas limit update log: data pointer")
            }
            Self::InvalidDataPointer(pointer) => {
                write!(f, "Invalid config update log: invalid data pointer: {}", pointer)
            }
            Self::LengthDecodingError => {
                write!(f, "Failed to decode gas limit update log: data length")
            }
            Self::InvalidDataLength(length) => {
                write!(f, "Invalid config update log: invalid data length: {}", length)
            }
            Self::GasLimitDecodingError => {
                write!(f, "Failed to decode gas limit update log: gas limit")
            }
        }
    }
}

impl core::error::Error for GasLimitUpdateError {}

/// An error for updating the EIP-1559 parameters on the [SystemConfig].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BaseFeeUpdateError {
    /// Invalid data length.
    InvalidDataLen(usize),
    /// Failed to decode the data pointer argument from the eip 1559 update log.
    PointerDecodingError,
    /// The data pointer is invalid.
    InvalidDataPointer(u64),
    /// Failed to decode the data length argument from the eip 1559 update log.
    LengthDecodingError,
    /// The data length is invalid.
    InvalidDataLength(u64),
    /// Failed to decode the eip1559 params argument from the eip 1559 update log.
    BaseFeeDecodingError,
}

impl core::fmt::Display for BaseFeeUpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDataLen(len) => {
                write!(f, "Invalid config update log: invalid data length: {}", len)
            }
            Self::PointerDecodingError => {
                write!(f, "Failed to decode eip1559 parameter update log: data pointer")
            }
            Self::InvalidDataPointer(pointer) => {
                write!(f, "Invalid config update log: invalid data pointer: {}", pointer)
            }
            Self::LengthDecodingError => {
                write!(f, "Failed to decode eip1559 parameter update log: data length")
            }
            Self::InvalidDataLength(length) => {
                write!(f, "Invalid config update log: invalid data length: {}", length)
            }
            Self::BaseFeeDecodingError => {
                write!(f, "Failed to decode base fee parameter update log: base fee invalid")
            }
        }
    }
}

impl core::error::Error for BaseFeeUpdateError {}

/// System accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SystemAccounts {
    /// The address that can deposit attributes
    pub attributes_depositor: Address,
    /// The address of the attributes predeploy
    pub attributes_predeploy: Address,
    /// The address of the fee vault
    pub fee_vault: Address,
}

impl Default for SystemAccounts {
    fn default() -> Self {
        Self {
            attributes_depositor: address!("deaddeaddeaddeaddeaddeaddeaddeaddead0001"),
            attributes_predeploy: address!("4200000000000000000000000000000000000015"),
            fee_vault: address!("4200000000000000000000000000000000000011"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;
    use alloy_primitives::{b256, hex, LogData, B256};
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn test_arbitrary_system_config() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());
        SystemConfig::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_system_config_serde() {
        let sc_str = r#"{
          "batcherAddr": "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985",
          "overhead": "0x00000000000000000000000000000000000000000000000000000000000000bc",
          "scalar": "0x00000000000000000000000000000000000000000000000000000000000a6fe0",
          "gasLimit": 30000000
        }"#;
        let system_config: SystemConfig = serde_json::from_str(sc_str).unwrap();
        assert_eq!(
            system_config.batcher_address,
            address!("6887246668a3b87F54DeB3b94Ba47a6f63F32985")
        );
        assert_eq!(system_config.overhead, U256::from(0xbc));
        assert_eq!(system_config.scalar, U256::from(0xa6fe0));
        assert_eq!(system_config.gas_limit, 30000000);
    }

    #[test]
    fn test_system_config_update_with_receipts_unchanged() {
        let mut system_config = SystemConfig::default();
        let receipts = vec![];
        let l1_system_config_address = Address::ZERO;
        let ecotone_active = false;

        system_config.update_with_receipts(&receipts, l1_system_config_address).unwrap();

        assert_eq!(system_config, SystemConfig::default());
    }

    #[test]
    fn test_system_config_update_with_receipts_batcher_address() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000000");
        let mut system_config = SystemConfig::default();
        let l1_system_config_address = Address::ZERO;
        let ecotone_active = false;

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        let receipt = Receipt {
            logs: vec![update_log],
            status: Eip658Value::Eip658(true),
            cumulative_gas_used: 0_u64,
        };

        system_config.update_with_receipts(&[receipt], l1_system_config_address).unwrap();

        assert_eq!(
            system_config.batcher_address,
            address!("000000000000000000000000000000000000bEEF"),
        );
    }

    #[test]
    fn test_system_config_update_batcher_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000000");

        let mut system_config = SystemConfig::default();

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the batcher address.
        system_config.process_config_update_log(&update_log).unwrap();

        assert_eq!(
            system_config.batcher_address,
            address!("000000000000000000000000000000000000bEEF")
        );
    }

    #[test]
    fn test_system_config_update_gas_config_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000001");

        let mut system_config = SystemConfig::default();

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000babe000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the batcher address.
        system_config.process_config_update_log(&update_log).unwrap();

        assert_eq!(system_config.overhead, U256::from(0xbabe));
        assert_eq!(system_config.scalar, U256::from(0xbeef));
    }


    #[test]
    fn test_system_config_update_gas_limit_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000002");

        let mut system_config = SystemConfig::default();

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the gas limit.
        system_config.process_config_update_log(&update_log).unwrap();

        assert_eq!(system_config.gas_limit, 0xbeef_u64);
    }

    #[test]
    fn test_system_config_update_eip1559_params_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000004");

        let mut system_config = SystemConfig::default();
        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000babe0000beef").into()
            )
        };
    }
}
