//! Module containing a [TxDeposit] builder for the Tenrec network upgrade transactions.
//!
//! Tenrec network upgrade transactions are defined in the [OP Stack Specs][specs].
//!
//! [specs]: https://specs.optimism.io/protocol/tenrec/derivation.html#network-upgrade-automation-transactions

use alloc::{string::String, vec::Vec};
use alloy_eips::eip2718::Encodable2718;
use alloy_primitives::{Address, B256, Bytes, TxKind, U256, address, hex};
use kona_protocol::Predeploys;
use op_alloy_consensus::{TxDeposit, UpgradeDepositSource};

use crate::Hardfork;

/// The Tenrec network upgrade transactions.
#[derive(Debug, Default, Clone, Copy)]
pub struct Tenrec;

impl Tenrec {
    /// EIP-2935 From Address
    pub const EIP2935_FROM: Address = address!("3462413Af4609098e1E27A490f554f260213D685");

    /// Returns the source hash for the EIP-2935 block hash history contract deployment.
    pub fn block_hash_history_contract_source() -> B256 {
        UpgradeDepositSource { intent: String::from("Tenrec: EIP-2935 Contract Deployment") }
            .source_hash()
    }

    /// Returns the sourec hash for the L2toL1MessagePasser update.
    pub fn deploy_l2_to_l1_message_passer_source() -> B256 {
        UpgradeDepositSource { intent: String::from("Tenrec: L2toL1MessagePasser Deployment") }
            .source_hash()
    }

    /// Returns the source hash for the L2toL1MessagePasser update.
    pub fn update_l2_to_l1_message_passer_source() -> B256 {
        UpgradeDepositSource { intent: String::from("Tenrec: L2toL1MessagePasser Proxy Update") }
            .source_hash()
    }

    /// Returns the deployment data for the L2toL1MessagePasser
    pub fn l2_to_l1_message_passer_deployment_data() -> Bytes {
        hex::decode(include_str!("./bytecode/l2_to_l1_message_passer_tenrec.hex").replace("\n", ""))
            .expect("Expected hex byte string")
            .into()
    }

    /// Returns the EIP-2935 creation data.
    pub fn eip2935_creation_data() -> Bytes {
        hex::decode(include_str!("./bytecode/eip2935_isthmus.hex").replace("\n", ""))
            .expect("Expected hex byte string")
            .into()
    }

    /// Returns the list of [TxDeposit]s for the network upgrade.
    pub fn deposits() -> impl Iterator<Item = TxDeposit> {
        ([
            TxDeposit {
                source_hash: Self::block_hash_history_contract_source(),
                from: Self::EIP2935_FROM,
                to: TxKind::Create,
                mint: 0,
                value: U256::ZERO,
                gas_limit: 250_000,
                is_system_transaction: false,
                input: Self::eip2935_creation_data(),
            },
            TxDeposit {
                source_hash: Self::deploy_l2_to_l1_message_passer_source(),
                from: Predeploys::L2_TO_L1_MESSAGE_DEPLOYER_PASSER,
                to: TxKind::Create,
                mint: 0,
                value: U256::ZERO,
                gas_limit: 2_000_000,
                is_system_transaction: false,
                input: Self::l2_to_l1_message_passer_deployment_data(),
            },
            TxDeposit {
                source_hash: Self::update_l2_to_l1_message_passer_source(),
                from: Address::ZERO,
                to: TxKind::Call(Predeploys::L2_TO_L1_MESSAGE_PASSER),
                mint: 0,
                value: U256::ZERO,
                gas_limit: 2_000_000,
                is_system_transaction: false,
                input: super::upgrade_to_and_call_calldata(
                    Predeploys::L2_TO_L1_MESSAGE_DEPLOYER_PASSER.create(0),
                ),
            },
        ])
        .into_iter()
    }
}

impl Hardfork for Tenrec {
    /// Constructs the network upgrade transactions.
    fn txs(&self) -> impl Iterator<Item = Bytes> + '_ {
        Self::deposits().map(|tx| {
            let mut encoded = Vec::new();
            tx.encode_2718(&mut encoded);
            Bytes::from(encoded)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_tenrec_txs_encoded() {
        let tenrec_upgrade_tx = Tenrec.txs().collect::<Vec<_>>();
        assert_eq!(tenrec_upgrade_tx.len(), 3);

        let expected_txs: Vec<Bytes> = vec![
            hex::decode(include_str!("./bytecode/tenrec_tx_0.hex").replace("\n", ""))
                .unwrap()
                .into(),
            hex::decode(include_str!("./bytecode/tenrec_tx_1.hex").replace("\n", ""))
                .unwrap()
                .into(),
            hex::decode(include_str!("./bytecode/tenrec_tx_2.hex").replace("\n", ""))
                .unwrap()
                .into(),
        ];
        for (i, expected) in expected_txs.iter().enumerate() {
            assert_eq!(tenrec_upgrade_tx[i], *expected);
        }
    }
}
