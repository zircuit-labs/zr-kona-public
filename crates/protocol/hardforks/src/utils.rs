//! Utilities for creating hardforks.

use alloy_primitives::{Address, Bytes, hex};
use alloy_sol_types::{SolCall, sol};

/// UpgradeTo Function 4Byte Signature
pub(crate) const UPGRADE_TO_FUNC_BYTES_4: [u8; 4] = hex!("3659cfe6");

sol!(
    /// L2ToL1MessagePasserMetadata contains all meta data concerning the L2ToL1MessagePasser contract.
    L2ToL1MessagePasserMetadata,
    r#"[{"type":"constructor","inputs":[],"stateMutability":"nonpayable"},{"type":"receive","stateMutability":"payable"},{"type":"function","name":"MESSAGE_VERSION","inputs":[],"outputs":[{"name":"","type":"uint16","internalType":"uint16"}],"stateMutability":"view"},{"type":"function","name":"accessController","inputs":[],"outputs":[{"name":"","type":"address","internalType":"contractAccessControlPausable"}],"stateMutability":"view"},{"type":"function","name":"initialize","inputs":[],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"initiateWithdrawal","inputs":[{"name":"_target","type":"address","internalType":"address"},{"name":"_gasLimit","type":"uint256","internalType":"uint256"},{"name":"_data","type":"bytes","internalType":"bytes"}],"outputs":[],"stateMutability":"payable"},{"type":"function","name":"messageNonce","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"paused","inputs":[],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"sentMessages","inputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"version","inputs":[],"outputs":[{"name":"","type":"string","internalType":"string"}],"stateMutability":"view"},{"type":"event","name":"Initialized","inputs":[{"name":"version","type":"uint64","indexed":false,"internalType":"uint64"}],"anonymous":false},{"type":"event","name":"MessagePassed","inputs":[{"name":"nonce","type":"uint256","indexed":true,"internalType":"uint256"},{"name":"sender","type":"address","indexed":true,"internalType":"address"},{"name":"target","type":"address","indexed":true,"internalType":"address"},{"name":"value","type":"uint256","indexed":false,"internalType":"uint256"},{"name":"gasLimit","type":"uint256","indexed":false,"internalType":"uint256"},{"name":"data","type":"bytes","indexed":false,"internalType":"bytes"},{"name":"withdrawalHash","type":"bytes32","indexed":false,"internalType":"bytes32"}],"anonymous":false},{"type":"error","name":"InvalidInitialization","inputs":[]},{"type":"error","name":"NotInitializing","inputs":[]}]"#,
);

sol! {
    /// UpgradeToAndCall Function
    function upgradeToAndCall(address, bytes);
}

/// Turns the given address into calldata for the `upgradeTo` function.
pub(crate) fn upgrade_to_calldata(addr: Address) -> Bytes {
    let mut v = UPGRADE_TO_FUNC_BYTES_4.to_vec();
    v.extend_from_slice(addr.into_word().as_slice());
    v.into()
}

/// Turns the given address and data into calldata for the `upgradeToAndCall` function.
pub(crate) fn upgrade_to_and_call_calldata(addr: Address) -> Bytes {
    let calldata = L2ToL1MessagePasserMetadata::initializeCall {}.abi_encode();

    upgradeToAndCallCall { _0: addr, _1: calldata.into() }.abi_encode().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ecotone, Fjord, Isthmus};
    use alloy_primitives::keccak256;

    #[test]
    fn test_upgrade_to_selector_is_valid() {
        let expected_selector = &keccak256("upgradeTo(address)")[..4];
        assert_eq!(UPGRADE_TO_FUNC_BYTES_4, expected_selector);
    }

    #[test]
    fn test_upgrade_to_calldata_format() {
        let test_addr = Address::from([0x42; 20]);
        let calldata = upgrade_to_calldata(test_addr);

        assert_eq!(calldata.len(), 36);
        assert_eq!(&calldata[..4], UPGRADE_TO_FUNC_BYTES_4);
        assert_eq!(&calldata[4..36], test_addr.into_word().as_slice());
    }

    #[test]
    fn test_ecotone_selector_is_valid() {
        let expected_selector = &keccak256("setEcotone()")[..4];
        assert_eq!(Ecotone::ENABLE_ECOTONE_INPUT, expected_selector);
    }

    #[test]
    fn test_fjord_selector_is_valid() {
        let expected_selector = &keccak256("setFjord()")[..4];
        assert_eq!(Fjord::SET_FJORD_METHOD_SIGNATURE, expected_selector);
    }

    #[test]
    fn test_isthmus_selector_is_valid() {
        let expected_selector = &keccak256("setIsthmus()")[..4];
        assert_eq!(Isthmus::ENABLE_ISTHMUS_INPUT, expected_selector);
    }
}
