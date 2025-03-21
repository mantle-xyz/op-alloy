//! Contains deposit transaction types and helper methods.

use alloc::{string::String, vec::Vec};
use alloy_primitives::{b256, keccak256, Address, Bytes, Log, TxKind, B256, U256, U64};
use core::fmt::Display;
use op_alloy_consensus::TxDeposit;

/// Deposit log event abi signature.
pub const DEPOSIT_EVENT_ABI: &str = "TransactionDeposited(address,address,uint256,bytes)";

/// Deposit event abi hash.
///
/// This is the keccak256 hash of the deposit event ABI signature.
/// `keccak256("TransactionDeposited(address,address,uint256,bytes)")`
pub const DEPOSIT_EVENT_ABI_HASH: B256 =
    b256!("b3813568d9991fc951961fcb4c784893574240a28925604d09fc577c55bb7c32");

/// The initial version of the deposit event log.
pub const DEPOSIT_EVENT_VERSION_0: B256 = B256::ZERO;

/// The version for mantle native token use mnt
pub const DEPOSIT_EVENT_VERSION_1: B256 =
    b256!("0000000000000000000000000000000000000000000000000000000000000001");

/// An [op_alloy_consensus::TxDeposit] validation error.
#[derive(Debug, PartialEq, Eq)]
pub enum DepositError {
    /// Unexpected number of deposit event log topics.
    UnexpectedTopicsLen(usize),
    /// Invalid deposit event selector.
    /// Expected: [B256] (deposit event selector), Actual: [B256] (event log topic).
    InvalidSelector(B256, B256),
    /// Incomplete opaqueData slice header (incomplete length).
    IncompleteOpaqueData(usize),
    /// The log data is not aligned to 32 bytes.
    UnalignedData(usize),
    /// Failed to decode the `from` field of the deposit event (the second topic).
    FromDecode(B256),
    /// Failed to decode the `to` field of the deposit event (the third topic).
    ToDecode(B256),
    /// Invalid opaque data content offset.
    InvalidOpaqueDataOffset(Bytes),
    /// Invalid opaque data content length.
    InvalidOpaqueDataLength(Bytes),
    /// Opaque data length exceeds the deposit log event data length.
    /// Specified: [usize] (data length), Actual: [usize] (opaque data length).
    OpaqueDataOverflow(usize, usize),
    /// Opaque data with padding exceeds the specified data length.
    PaddedOpaqueDataOverflow(usize, usize),
    /// An invalid deposit version.
    InvalidVersion(B256),
    /// Unexpected opaque data length
    UnexpectedOpaqueDataLen(usize),
    /// Failed to decode the deposit mint value.
    MintDecode(Bytes),
    /// Failed to decode the deposit gas value.
    GasDecode(Bytes),
    /// Failed to decode the deposit mint eth value
    EthValueDecode(Bytes),
    /// Failed to decode the deposit eth tx value
    EthTxValueDecode(Bytes),
}

impl core::error::Error for DepositError {}

impl Display for DepositError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnexpectedTopicsLen(len) => {
                write!(f, "Unexpected number of deposit event log topics: {}", len)
            }
            Self::InvalidSelector(expected, actual) => {
                write!(f, "Invalid deposit event selector: {}, expected {}", actual, expected)
            }
            Self::IncompleteOpaqueData(len) => {
                write!(f, "Incomplete opaqueData slice header (incomplete length): {}", len)
            }
            Self::UnalignedData(data) => {
                write!(f, "Unaligned log data, expected multiple of 32 bytes, got: {}", data)
            }
            Self::FromDecode(topic) => {
                write!(f, "Failed to decode the `from` address of the deposit log topic: {}", topic)
            }
            Self::ToDecode(topic) => {
                write!(f, "Failed to decode the `to` address of the deposit log topic: {}", topic)
            }
            Self::InvalidOpaqueDataOffset(offset) => {
                write!(f, "Invalid u64 opaque data content offset: {:?}", offset)
            }
            Self::InvalidOpaqueDataLength(length) => {
                write!(f, "Invalid u64 opaque data content length: {:?}", length)
            }
            Self::OpaqueDataOverflow(data_len, opaque_len) => {
                write!(
                    f,
                    "Specified opaque data length {} exceeds the deposit log event data length {}",
                    opaque_len, data_len
                )
            }
            Self::PaddedOpaqueDataOverflow(data_len, opaque_len) => {
                write!(
                    f,
                    "Opaque data with padding exceeds the specified data length: {} > {}",
                    opaque_len, data_len
                )
            }
            Self::InvalidVersion(version) => {
                write!(f, "Invalid deposit version: {}", version)
            }
            Self::UnexpectedOpaqueDataLen(len) => {
                write!(f, "Unexpected opaque data length: {}", len)
            }
            Self::MintDecode(data) => {
                write!(f, "Failed to decode the u128 deposit mint value: {:?}", data)
            }
            Self::GasDecode(data) => {
                write!(f, "Failed to decode the u64 deposit gas value: {:?}", data)
            }
            Self::EthValueDecode(data) => {
                write!(f, "Failed to decode the u128 deposit eth value: {:?}", data)
            }
            Self::EthTxValueDecode(data) => {
                write!(f, "Failed to decode the u128 deposit eth tx value: {:?}", data)
            }
        }
    }
}

/// Source domain identifiers for deposit transactions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
#[repr(u8)]
pub enum DepositSourceDomainIdentifier {
    /// A user deposit source.
    User = 0,
    /// A L1 info deposit source.
    L1Info = 1,
    /// An upgrade deposit source.
    Upgrade = 2,
}

/// Source domains for deposit transactions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DepositSourceDomain {
    /// A user deposit source.
    User(UserDepositSource),
    /// A L1 info deposit source.
    L1Info(L1InfoDepositSource),
    /// An upgrade deposit source.
    Upgrade(UpgradeDepositSource),
}

impl DepositSourceDomain {
    /// Returns the source hash.
    pub fn source_hash(&self) -> B256 {
        match self {
            Self::User(ds) => ds.source_hash(),
            Self::L1Info(ds) => ds.source_hash(),
            Self::Upgrade(ds) => ds.source_hash(),
        }
    }
}

/// A deposit transaction source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct UserDepositSource {
    /// The L1 block hash.
    pub l1_block_hash: B256,
    /// The log index.
    pub log_index: u64,
}

impl UserDepositSource {
    /// Creates a new [UserDepositSource].
    pub const fn new(l1_block_hash: B256, log_index: u64) -> Self {
        Self { l1_block_hash, log_index }
    }

    /// Returns the source hash.
    pub fn source_hash(&self) -> B256 {
        let mut input = [0u8; 32 * 2];
        input[..32].copy_from_slice(&self.l1_block_hash[..]);
        input[32 * 2 - 8..].copy_from_slice(&self.log_index.to_be_bytes());
        let deposit_id_hash = keccak256(input);
        let mut domain_input = [0u8; 32 * 2];
        let identifier_bytes: [u8; 8] = (DepositSourceDomainIdentifier::User as u64).to_be_bytes();
        domain_input[32 - 8..32].copy_from_slice(&identifier_bytes);
        domain_input[32..].copy_from_slice(&deposit_id_hash[..]);
        keccak256(domain_input)
    }
}

/// A L1 info deposit transaction source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct L1InfoDepositSource {
    /// The L1 block hash.
    pub l1_block_hash: B256,
    /// The sequence number.
    pub seq_number: u64,
}

impl L1InfoDepositSource {
    /// Creates a new [L1InfoDepositSource].
    pub const fn new(l1_block_hash: B256, seq_number: u64) -> Self {
        Self { l1_block_hash, seq_number }
    }

    /// Returns the source hash.
    pub fn source_hash(&self) -> B256 {
        let mut input = [0u8; 32 * 2];
        input[..32].copy_from_slice(&self.l1_block_hash[..]);
        input[32 * 2 - 8..].copy_from_slice(&self.seq_number.to_be_bytes());
        let deposit_id_hash = keccak256(input);
        let mut domain_input = [0u8; 32 * 2];
        let identifier_bytes: [u8; 8] =
            (DepositSourceDomainIdentifier::L1Info as u64).to_be_bytes();
        domain_input[32 - 8..32].copy_from_slice(&identifier_bytes);
        domain_input[32..].copy_from_slice(&deposit_id_hash[..]);
        keccak256(domain_input)
    }
}

/// An upgrade deposit transaction source.
///
/// This implements the translation of upgrade-tx identity information to a deposit source-hash,
/// which makes the deposit uniquely identifiable.
/// System-upgrade transactions have their own domain for source-hashes,
/// to not conflict with user-deposits or deposited L1 information.
/// The intent identifies the upgrade-tx uniquely, in a human-readable way.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpgradeDepositSource {
    /// The intent.
    pub intent: String,
}

impl UpgradeDepositSource {
    /// Creates a new [UpgradeDepositSource].
    pub const fn new(intent: String) -> Self {
        Self { intent }
    }

    /// Returns the source hash.
    pub fn source_hash(&self) -> B256 {
        let intent_hash = keccak256(self.intent.as_bytes());
        let mut domain_input = [0u8; 32 * 2];
        let identifier_bytes: [u8; 8] =
            (DepositSourceDomainIdentifier::Upgrade as u64).to_be_bytes();
        domain_input[32 - 8..32].copy_from_slice(&identifier_bytes);
        domain_input[32..].copy_from_slice(&intent_hash[..]);
        keccak256(domain_input)
    }
}

/// Derives a deposit transaction from an EVM log event emitted by the deposit contract.
///
/// The emitted log must be in format:
/// ```solidity
/// event TransactionDeposited(
///    address indexed from,
///    address indexed to,
///    uint256 indexed version,
///    bytes opaqueData
/// );
/// ```
pub fn decode_deposit(block_hash: B256, index: usize, log: &Log) -> Result<Bytes, DepositError> {
    let topics = log.data.topics();
    if topics.len() != 4 {
        return Err(DepositError::UnexpectedTopicsLen(topics.len()));
    }
    if topics[0] != DEPOSIT_EVENT_ABI_HASH {
        return Err(DepositError::InvalidSelector(DEPOSIT_EVENT_ABI_HASH, topics[0]));
    }
    if log.data.data.len() < 64 {
        return Err(DepositError::IncompleteOpaqueData(log.data.data.len()));
    }
    if log.data.data.len() % 32 != 0 {
        return Err(DepositError::UnalignedData(log.data.data.len()));
    }

    let from = Address::try_from(&topics[1].as_slice()[12..])
        .map_err(|_| DepositError::FromDecode(topics[1]))?;
    let to = Address::try_from(&topics[2].as_slice()[12..])
        .map_err(|_| DepositError::ToDecode(topics[2]))?;
    let version = log.data.topics()[3];

    // Solidity serializes the event's Data field as follows:
    //
    // ```solidity
    // abi.encode(abi.encodPacked(uint256 mint, uint256 value, uint64 gasLimit, uint8 isCreation, bytes data))
    // ```
    //
    // The the opaqueData will be packed as shown below:
    //
    // ------------------------------------------------------------
    // | offset | 256 byte content                                |
    // ------------------------------------------------------------
    // | 0      | [0; 24] . {U64 big endian, hex encoded offset}  |
    // ------------------------------------------------------------
    // | 32     | [0; 24] . {U64 big endian, hex encoded length}  |
    // ------------------------------------------------------------

    let opaque_content_offset: U64 = U64::try_from_be_slice(&log.data.data[24..32]).ok_or(
        DepositError::InvalidOpaqueDataOffset(Bytes::copy_from_slice(&log.data.data[24..32])),
    )?;
    if opaque_content_offset != U64::from(32) {
        return Err(DepositError::InvalidOpaqueDataOffset(Bytes::copy_from_slice(
            &log.data.data[24..32],
        )));
    }

    // The next 32 bytes indicate the length of the opaqueData content.
    let opaque_content_len =
        u64::from_be_bytes(log.data.data[56..64].try_into().map_err(|_| {
            DepositError::InvalidOpaqueDataLength(Bytes::copy_from_slice(&log.data.data[56..64]))
        })?);
    if opaque_content_len as usize > log.data.data.len() - 64 {
        return Err(DepositError::OpaqueDataOverflow(
            opaque_content_len as usize,
            log.data.data.len() - 64,
        ));
    }
    let padded_len = opaque_content_len.checked_add(32).ok_or(DepositError::OpaqueDataOverflow(
        opaque_content_len as usize,
        log.data.data.len() - 64,
    ))?;
    if padded_len as usize <= log.data.data.len() - 64 {
        return Err(DepositError::PaddedOpaqueDataOverflow(
            log.data.data.len() - 64,
            opaque_content_len as usize,
        ));
    }

    // The remaining data is the opaqueData which is tightly packed and then padded to 32 bytes by
    // the EVM.
    let opaque_data = &log.data.data[64..64 + opaque_content_len as usize];
    let source = UserDepositSource::new(block_hash, index as u64);

    let mut deposit_tx = TxDeposit {
        from,
        is_system_transaction: false,
        source_hash: source.source_hash(),
        ..Default::default()
    };

    match version {
        DEPOSIT_EVENT_VERSION_0 => unmarshal_deposit_version0(&mut deposit_tx, to, opaque_data)?,
        DEPOSIT_EVENT_VERSION_1 => unmarshal_deposit_version1(&mut deposit_tx, to, opaque_data)?,
        _ => return Err(DepositError::InvalidVersion(version)),
    }

    // Re-encode the deposit transaction
    let mut buffer = Vec::with_capacity(deposit_tx.eip2718_encoded_length());
    deposit_tx.eip2718_encode(&mut buffer);
    Ok(Bytes::from(buffer))
}

/// Unmarshals a deposit transaction from the opaque data.
pub(crate) fn unmarshal_deposit_version0(
    tx: &mut TxDeposit,
    to: Address,
    data: &[u8],
) -> Result<(), DepositError> {
    if data.len() < 32 + 32 + 32 + 8 + 1 {
        return Err(DepositError::UnexpectedOpaqueDataLen(data.len()));
    }

    let mut offset = 0;

    // u128 mint mnt value
    tx.mint = decode_u128_field(data, offset, DepositError::MintDecode)?;
    offset += 32;

    // uint256 value
    tx.value = U256::from_be_slice(&data[offset..offset + 32]);
    offset += 32;

    // u128 mint eth_value
    tx.eth_value = decode_u128_field(data, offset, DepositError::EthValueDecode)?;
    offset += 32;

    // uint64 gas
    tx.gas_limit = decode_u8_field(data, offset, DepositError::GasDecode)?;
    offset += 8;

    // uint8 isCreation
    // isCreation: If the boolean byte is 1 then dep.To will stay nil,
    // and it will create a contract using L2 account nonce to determine the created address.
    if data[offset] == 0 {
        tx.to = TxKind::Call(to);
    } else {
        tx.to = TxKind::Create;
    }
    offset += 1;

    // The remainder of the opaqueData is the transaction data (without length prefix).
    // The data may be padded to a multiple of 32 bytes
    let tx_data_len = data.len() - offset;

    // Remaining bytes fill the data
    tx.input = Bytes::copy_from_slice(&data[offset..offset + tx_data_len]);

    Ok(())
}

/// Unmarshals a deposit transaction from the opaque data in deposit version1.
pub(crate) fn unmarshal_deposit_version1(
    tx: &mut TxDeposit,
    to: Address,
    data: &[u8],
) -> Result<(), DepositError> {
    if data.len() < 32 + 32 + 32 + 32 + 8 + 1 {
        return Err(DepositError::UnexpectedOpaqueDataLen(data.len()));
    }

    let mut offset = 0;

    // u128 mint mnt value
    tx.mint = decode_u128_field(data, offset, DepositError::MintDecode)?;
    offset += 32;

    // uint256 value
    tx.value = U256::from_be_slice(&data[offset..offset + 32]);
    offset += 32;

    // u128 mint eth_value
    tx.eth_value = decode_u128_field(data, offset, DepositError::EthValueDecode)?;
    offset += 32;

    // uint256 eth_tx_value
    tx.eth_tx_value = decode_u128_field(data, offset, DepositError::EthTxValueDecode)?;
    offset += 32;

    // uint64 gas
    tx.gas_limit = decode_u8_field(data, offset, DepositError::GasDecode)?;
    offset += 8;

    // uint8 isCreation
    // isCreation: If the boolean byte is 1 then dep.To will stay nil,
    // and it will create a contract using L2 account nonce to determine the created address.
    if data[offset] == 0 {
        tx.to = TxKind::Call(to);
    } else {
        tx.to = TxKind::Create;
    }
    offset += 1;

    // The remainder of the opaqueData is the transaction data (without length prefix).
    // The data may be padded to a multiple of 32 bytes
    let tx_data_len = data.len() - offset;

    // Remaining bytes fill the data
    tx.input = Bytes::copy_from_slice(&data[offset..offset + tx_data_len]);

    Ok(())
}

/// decode u128 field function
pub fn decode_u128_field<E>(
    data: &[u8],
    offset: usize,
    error_fn: impl FnOnce(Bytes) -> E,
) -> Result<Option<u128>, E> {
    let raw_value: [u8; 16] = data[offset + 16..offset + 32]
        .try_into()
        .map_err(|_| error_fn(Bytes::copy_from_slice(&data[offset + 16..offset + 32])))?;
    let value = u128::from_be_bytes(raw_value);
    Ok(if value == 0 { None } else { Some(value) })
}

/// decode u8 field function
pub fn decode_u8_field<E>(
    data: &[u8],
    offset: usize,
    error_fn: impl FnOnce(Bytes) -> E,
) -> Result<u64, E> {
    let raw_value: [u8; 8] = data[offset..offset + 8]
        .try_into()
        .map_err(|_| error_fn(Bytes::copy_from_slice(&data[offset..offset + 8])))?;
    Ok(u64::from_be_bytes(raw_value))
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;
    use alloy_primitives::{address, b256, hex, LogData};

    #[test]
    fn test_decode_deposit_invalid_first_topic() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![B256::default(), B256::default(), B256::default(), B256::default()],
                Bytes::default(),
            ),
        };
        let err: DepositError = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::InvalidSelector(DEPOSIT_EVENT_ABI_HASH, B256::default()));
    }

    #[test]
    fn test_decode_deposit_incomplete_data() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(vec![0u8; 63]),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::IncompleteOpaqueData(63));
    }

    #[test]
    fn test_decode_deposit_unaligned_data() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(vec![0u8; 65]),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::UnalignedData(65));
    }

    #[test]
    #[ignore]
    const fn test_decode_deposit_invalid_from() {}

    #[test]
    #[ignore]
    const fn test_decode_deposit_invalid_to() {}

    #[test]
    fn test_decode_deposit_invalid_opaque_data_offset() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(vec![0u8; 64]),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::InvalidOpaqueDataOffset(Bytes::from(vec![0u8; 8])));
    }

    #[test]
    fn test_decode_deposit_opaque_data_overflow() {
        let mut data = vec![0u8; 128];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        // The first 64 bytes of the data are identifiers so
        // if this test was to be valid, the data length would be 64 not 128.
        let len: [u8; 8] = U64::from(128).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(data),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::OpaqueDataOverflow(128, 64));
    }

    #[test]
    fn test_decode_deposit_padded_overflow() {
        let mut data = vec![0u8; 256];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(64).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(data),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::PaddedOpaqueDataOverflow(192, 64));
    }

    #[test]
    fn test_decode_deposit_invalid_version() {
        let mut data = vec![0u8; 128];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(64).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        let version = b256!("0000000000000000000000000000000000000000000000000000000000000001");
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), version],
                Bytes::from(data),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::InvalidVersion(version));
    }

    #[test]
    fn test_decode_deposit_empty_succeeds() {
        let mut data = vec![0u8; 192];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(128).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(data),
            ),
        };
        let tx = decode_deposit(B256::default(), 0, &log).unwrap();
        let raw_hex = hex!("7ef887a0ed428e1c45e1d9561b62834e1a2d3015a0caae3bfdc16b4da059ac885b01a14594000000000000000000000000000000000000000094000000000000000000000000000000000000000080808080b700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let expected = Bytes::from(raw_hex);
        assert_eq!(tx, expected);
    }

    #[test]
    fn test_decode_deposit_full_succeeds() {
        let mut data = vec![0u8; 192];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(128).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        // Copy the u128 mint value
        let mint: [u8; 16] = 10_u128.to_be_bytes();
        data[80..96].copy_from_slice(&mint);
        // Copy the tx value
        let value: [u8; 32] = U256::from(100).to_be_bytes();
        data[96..128].copy_from_slice(&value);
        // Copy the gas limit
        let gas: [u8; 8] = 1000_u64.to_be_bytes();
        data[128..136].copy_from_slice(&gas);
        // Copy the isCreation flag
        data[136] = 1;
        let from = address!("1111111111111111111111111111111111111111");
        let mut from_bytes = vec![0u8; 32];
        from_bytes[12..32].copy_from_slice(from.as_slice());
        let to = address!("2222222222222222222222222222222222222222");
        let mut to_bytes = vec![0u8; 32];
        to_bytes[12..32].copy_from_slice(to.as_slice());
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![
                    DEPOSIT_EVENT_ABI_HASH,
                    B256::from_slice(&from_bytes),
                    B256::from_slice(&to_bytes),
                    B256::default(),
                ],
                Bytes::from(data),
            ),
        };
        let tx = decode_deposit(B256::default(), 0, &log).unwrap();
        let raw_hex = hex!("7ef875a0ed428e1c45e1d9561b62834e1a2d3015a0caae3bfdc16b4da059ac885b01a145941111111111111111111111111111111111111111800a648203e880b700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let expected = Bytes::from(raw_hex);
        assert_eq!(tx, expected);
    }

    #[test]
    fn test_unmarshal_deposit_version0_invalid_len() {
        let data = vec![0u8; 72];
        let mut tx = TxDeposit::default();
        let to = address!("5555555555555555555555555555555555555555");
        let err = unmarshal_deposit_version0(&mut tx, to, &data).unwrap_err();
        assert_eq!(err, DepositError::UnexpectedOpaqueDataLen(72));

        // Data must have at least length 73
        let data = vec![0u8; 73];
        let mut tx = TxDeposit::default();
        let to = address!("5555555555555555555555555555555555555555");
        unmarshal_deposit_version0(&mut tx, to, &data).unwrap();
    }

    #[test]
    fn test_unmarshal_deposit_version0() {
        let mut data = vec![0u8; 192];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(128).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        // Copy the u128 mint value
        let mint: [u8; 16] = 10_u128.to_be_bytes();
        data[80..96].copy_from_slice(&mint);
        // Copy the tx value
        let value: [u8; 32] = U256::from(100).to_be_bytes();
        data[96..128].copy_from_slice(&value);
        // Copy the eth value
        let eth_value: [u8; 16] = 10_u128.to_be_bytes();
        data[128..144].copy_from_slice(&eth_value);
        // Copy the gas limit
        let gas: [u8; 8] = 1000_u64.to_be_bytes();
        data[144..152].copy_from_slice(&gas);
        // Copy the isCreation flag
        data[152] = 1;
        let mut tx = TxDeposit {
            from: address!("1111111111111111111111111111111111111111"),
            to: TxKind::Call(address!("2222222222222222222222222222222222222222")),
            value: U256::from(100),
            gas_limit: 1000,
            mint: Some(10),
            eth_value: Some(20),
            ..Default::default()
        };
        let to = address!("5555555555555555555555555555555555555555");
        unmarshal_deposit_version0(&mut tx, to, &data).unwrap();
        assert_eq!(tx.to, TxKind::Call(address!("5555555555555555555555555555555555555555")));
    }

    #[test]
    fn test_unmarshal_deposit_version1() {
        let mut data = vec![0u8; 192];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(128).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        // Copy the u128 mint value
        let mint: [u8; 16] = 10_u128.to_be_bytes();
        data[80..96].copy_from_slice(&mint);
        // Copy the tx value
        let value: [u8; 32] = U256::from(100).to_be_bytes();
        data[96..128].copy_from_slice(&value);
        // Copy the eth value
        let eth_value: [u8; 16] = 10_u128.to_be_bytes();
        data[128..144].copy_from_slice(&eth_value);
        // Copy the eth tx value
        let eth_tx_value: [u8; 16] = 10_u128.to_be_bytes();
        data[144..160].copy_from_slice(&eth_tx_value);
        // Copy the gas limit
        let gas: [u8; 8] = 1000_u64.to_be_bytes();
        data[160..168].copy_from_slice(&gas);
        // Copy the isCreation flag
        data[168] = 1;
        let mut tx = TxDeposit {
            from: address!("1111111111111111111111111111111111111111"),
            to: TxKind::Call(address!("2222222222222222222222222222222222222222")),
            value: U256::from(100),
            gas_limit: 1000,
            mint: Some(10),
            eth_value: Some(20),
            eth_tx_value: Some(20),
            ..Default::default()
        };
        let to = address!("5555555555555555555555555555555555555555");
        unmarshal_deposit_version0(&mut tx, to, &data).unwrap();
        assert_eq!(tx.to, TxKind::Call(address!("5555555555555555555555555555555555555555")));
    }
}
