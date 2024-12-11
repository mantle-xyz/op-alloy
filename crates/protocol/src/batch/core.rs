//! Module containing the core [Batch] enum.

use crate::{BatchDecodingError, SingleBatch};
use alloy_rlp::{Buf, Decodable};
use op_alloy_genesis::RollupConfig;

/// A Batch.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Batch {
    /// A single batch
    Single(SingleBatch),
}

impl Batch {
    /// Returns the timestamp for the batch.
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::Single(sb) => sb.timestamp,
        }
    }

    /// Attempts to decode a batch from a reader.
    pub fn decode(r: &mut &[u8], cfg: &RollupConfig) -> Result<Self, BatchDecodingError> {
        if r.is_empty() {
            return Err(BatchDecodingError::EmptyBuffer);
        }

        r.advance(1);
        
        let single_batch =
            SingleBatch::decode(r).map_err(BatchDecodingError::AlloyRlpError)?;
        Ok(Self::Single(single_batch))
    }
}
