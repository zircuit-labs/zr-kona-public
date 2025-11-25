//! Sync Start

use crate::errors::OracleProviderError;
use alloc::sync::Arc;
use alloy_consensus::{Header, Sealed};
use alloy_primitives::B256;
use core::fmt::Debug;
use kona_derive::{
    PipelineError, PipelineErrorKind,
    ChainProvider, L2ChainProvider,
};
use kona_driver::{PipelineCursor, TipCursor};
use kona_genesis::{CONFIG_UPDATE_TOPIC, UPDATE_TYPE_BATCHER_TOPIC};
use kona_protocol::{BatchValidationProvider, L2BlockInfo};
use kona_registry::RollupConfig;
use spin::RwLock;

/// Computes the Cursor origin
pub fn compute_origin(
    rollup_config: &RollupConfig,
    l1_origin: u64,
    safe_head_info: L2BlockInfo,
) -> u64 {
    let channel_timeout = rollup_config.channel_timeout(safe_head_info.block_info.timestamp);
    let mut l1_origin_number = l1_origin.saturating_sub(channel_timeout);
    if l1_origin_number < rollup_config.genesis.l1.number {
        l1_origin_number = rollup_config.genesis.l1.number;
    }
    l1_origin_number
}

/// Constructs a [`PipelineCursor`] from the caching oracle, boot info, and providers.
pub async fn new_oracle_pipeline_cursor<L1, L2>(
    rollup_config: &RollupConfig,
    safe_header: Sealed<Header>,
    chain_provider: &mut L1,
    l2_chain_provider: &mut L2,
) -> Result<Arc<RwLock<PipelineCursor>>, OracleProviderError>
where
    L1: ChainProvider + Send + Sync + Debug + Clone,
    L2: BatchValidationProvider + Send + Sync + Debug + Clone,
    OracleProviderError:
        From<<L1 as ChainProvider>::Error> + From<<L2 as BatchValidationProvider>::Error>,
{
    let safe_head_info = l2_chain_provider.l2_block_info_by_number(safe_header.number).await?;
    let l1_origin = chain_provider.block_info_by_number(safe_head_info.l1_origin.number).await?;

    let l1_origin_number = compute_origin(rollup_config, l1_origin.number, safe_head_info);
    let channel_timeout = rollup_config.channel_timeout(safe_head_info.block_info.timestamp);
    let origin = chain_provider.block_info_by_number(l1_origin_number).await?;

    // Construct the cursor.
    let mut cursor = PipelineCursor::new(channel_timeout, origin);
    let tip = TipCursor::new(safe_head_info, safe_header, B256::ZERO);
    cursor.advance(origin, tip);

    // Wrap the cursor in a shared read-write lock
    Ok(Arc::new(RwLock::new(cursor)))
}

/// Checks for a batcher sender change between L1 Origin and L1 Head.
pub async fn has_batcher_sender_change<L1, L2>(
    sync_start: Arc<RwLock<PipelineCursor>>,
    chain_provider: &mut L1,
    l2_chain_provider: &mut L2,
    rollup_config: Arc<RollupConfig>,
) -> Result<bool, PipelineErrorKind>
where
    L1: ChainProvider + Send + Sync + Debug + Clone,
    L2: L2ChainProvider + Send + Sync + Debug + Clone,
{
    #[cfg(target_os = "zkvm")]
    println!("cycle-tracker-report-start: batch-sender-change-check");

    let mut system_config = l2_chain_provider
        .system_config_by_number(
            sync_start.read().l2_safe_head().block_info.number,
            rollup_config.clone(),
        )
        .await
        .map_err(Into::into)?;

    let mut block = sync_start.read().origin();

    loop {
        let block_header = chain_provider.header_by_hash(block.hash).await.map_err(Into::into)?;

        // TODO: False positive attack mitigation
        if block_header.logs_bloom.contains_raw_log(
            rollup_config.l1_system_config_address,
            &[CONFIG_UPDATE_TOPIC, UPDATE_TYPE_BATCHER_TOPIC],
        ) {
            tracing::error!("Block {:?} may contain log", block_header.number);

            let receipts = chain_provider.receipts_by_hash(block.hash).await.map_err(Into::into)?;

            let pre = system_config.batcher_address;

            if let Err(e) = system_config.update_with_receipts(
                receipts.as_slice(),
                rollup_config.l1_system_config_address,
                rollup_config.is_ecotone_active(block.timestamp),
            ) {
                return Err(PipelineError::SystemConfigUpdate(e).crit());
            }

            if pre != system_config.batcher_address {
                #[cfg(target_os = "zkvm")]
                println!("cycle-tracker-report-end: batch-sender-change-check");
                return Ok(true);
            }
        }

        block =
            match chain_provider.block_info_by_number(block.number + 1).await.map_err(Into::into) {
                Ok(block_info) => block_info,
                Err(PipelineErrorKind::Critical(PipelineError::EndOfSource)) => {
                    break;
                }
                Err(err) => {
                    return Err(err);
                }
            };
        // TODO: Add false positive counter
    }

    #[cfg(target_os = "zkvm")]
    println!("cycle-tracker-report-end: batch-sender-change-check");

    Ok(false)
}
