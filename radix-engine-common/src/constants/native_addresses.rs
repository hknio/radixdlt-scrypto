use crate::types::*;

//=========================================================================
// Please see and update REP-71 along with changes to this file
//=========================================================================

//=========================================================================
// FUNGIBLES
//=========================================================================

pub const DEFAULT_NEXT_NODE_IDS : [u32; 256] = {
    let mut arr = [1; 256];
    arr[13] += 15;
    arr[93] += 1;
    arr[130] += 1;
    arr[134] += 1;
    arr[154] += 9;
    arr[192] += 2;
    arr
};


/// XRD is the native token of the Radix ledger.
/// It is a fungible token, measured in attos (`10^-18`).
///
/// It is used for paying fees and staking.
pub const XRD: ResourceAddress = ResourceAddress::new_or_panic([
    93, 0, 1
]);

//=========================================================================
// VIRTUAL BADGES
//=========================================================================

/// The non-fungible badge resource which is used for virtual proofs of ECDSA Secp256k1 transacton signatures in the transaction processor.
pub const SECP256K1_SIGNATURE_VIRTUAL_BADGE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 1
]);

/// The non-fungible badge resource which is used for virtual proofs of EdDSA Ed25519 transacton signatures in the transaction processor.
pub const ED25519_SIGNATURE_VIRTUAL_BADGE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 2
]);

/// The non-fungible badge resource which is used for virtual proofs which represent the package of
/// the immediate caller - ie the actor which made the latest (global or internal) call.
///
/// For example, if there is a global component A containing an internal component A2, and A2 makes a global call to B,
/// then the access check for that global call will see a proof of this `PACKAGE_OF_DIRECT_CALLER_VIRTUAL_BADGE` for the package of A2.
pub const PACKAGE_OF_DIRECT_CALLER_VIRTUAL_BADGE: ResourceAddress =
    ResourceAddress::new_or_panic([
        154, 0, 3
    ]);

/// The non-fungible badge resource which is used for virtual proofs which represent the global ancestor
/// of the actor which made the latest global call.
///
/// For example, if there is a global component A containing an internal component A2, and A2 makes a global call to B,
/// then the access check for that global call will see a proof of this `GLOBAL_CALLER_VIRTUAL_BADGE` for the global component A.
pub const GLOBAL_CALLER_VIRTUAL_BADGE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 4
]);

//=========================================================================
// TRANSACTION BADGES
//=========================================================================

/// The non-fungible badge resource which is used for virtual proofs representing the fact that the current transaction is
/// a system transaction.
///
/// The following ids have meanings:
/// * `0` is used to represent a full-authority system transaction such as genesis, or a protocol update
/// * `1` is used to represent a consensus-authrority transaction, such as a round change
pub const SYSTEM_TRANSACTION_BADGE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 5
]);

//=========================================================================
// ENTITY OWNER BADGES
//=========================================================================

/// The non-fungible badge resource which is used for package ownership when creating packages with the simple package creation set-up.
pub const PACKAGE_OWNER_BADGE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 6
]);

/// The non-fungible badge resource which is used for validator ownership.
pub const VALIDATOR_OWNER_BADGE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 7
]);

/// The non-fungible badge resource which is used for account ownership, if accounts have been set up with simple account creation, or have been securified.
pub const ACCOUNT_OWNER_BADGE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 8
]);

/// The non-fungible badge resource which is used for identity ownership, if identities have been set up with simple account creation, or have been securified.
pub const IDENTITY_OWNER_BADGE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 9
]);

//=========================================================================
// PACKAGES
//=========================================================================

/// The native package for package deployment.
pub const PACKAGE_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 1
]);

/// The native package for resource managers, proofs, buckets, vaults etc.
pub const RESOURCE_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 2
]);

/// The native package for accounts.
pub const ACCOUNT_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 3
]);

/// The native package for identities.
pub const IDENTITY_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 4
]);

/// The native package for the consensus manager.
pub const CONSENSUS_MANAGER_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 5
]);

/// The native package for access controllers.
pub const ACCESS_CONTROLLER_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 6
]);

/// The native package for pools.
pub const POOL_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 7
]);

/// The native package for the transaction processor.
pub const TRANSACTION_PROCESSOR_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 8
]);

/// The native package for the metadata module.
pub const METADATA_MODULE_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 9
]);

/// The native package for the royalty module.
pub const ROYALTY_MODULE_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 10
]);

/// The native package for the role assignment module.
pub const ROLE_ASSIGNMENT_MODULE_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 11
]);

/// The native package for test utils.
pub const TEST_UTILS_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 12
]);

/// The scrypto package for the genesis helper.
pub const GENESIS_HELPER_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 13
]);
/// The name of the genesis helper blueprint under the `GENESIS_HELPER_PACKAGE`.
pub const GENESIS_HELPER_BLUEPRINT: &str = "GenesisHelper";

/// The scrypto package for the faucet
pub const FAUCET_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 14
]);
/// The name of the faucet blueprint under the `FAUCET_PACKAGE`.
pub const FAUCET_BLUEPRINT: &str = "Faucet";

pub const TRANSACTION_TRACKER_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 15
]);
pub const TRANSACTION_TRACKER_BLUEPRINT: &str = "TransactionTracker";

//=========================================================================
// SYSTEM SINGLETON COMPONENTS - NATIVE
//=========================================================================

/// The consensus manager native component - in charge of validators, consensus and epochs.
pub const CONSENSUS_MANAGER: ComponentAddress = ComponentAddress::new_or_panic([
    134, 0, 1
]);

//=========================================================================
// SYSTEM SINGLETON COMPONENTS - SCRYPTO
//=========================================================================

/// The genesis helper scrypto component - used for sorting out genesis.
pub const GENESIS_HELPER: ComponentAddress = ComponentAddress::new_or_panic([
    192, 0, 1
]);

/// The faucet native component - use this on testnets for getting XRD and locking fee.
pub const FAUCET: ComponentAddress = ComponentAddress::new_or_panic([
    192, 0, 2
]);
// Export an alias for backwards compatibility of dApp developer tests
pub use FAUCET as FAUCET_COMPONENT;

/// The intent hash store component
pub const TRANSACTION_TRACKER: ComponentAddress = ComponentAddress::new_or_panic([
    130, 0, 1
]);

//=========================================================================
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use radix_engine_common::{address::AddressBech32Encoder, network::NetworkDefinition};

    #[test]
    fn test_mainnet_vanity_addresses() {
        // Fungible Resources
        check_address(
            XRD.as_ref(),
            EntityType::GlobalFungibleResourceManager,
            "resource_rdx1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxradxrd",
        );

        // Virtual Badges
        check_address(
            SECP256K1_SIGNATURE_VIRTUAL_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxsecpsgxxxxxxxxx004638826440xxxxxxxxxsecpsg",
        );
        check_address(
            ED25519_SIGNATURE_VIRTUAL_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxed25sgxxxxxxxxx002236757237xxxxxxxxxed25sg",
        );
        check_address(
            PACKAGE_OF_DIRECT_CALLER_VIRTUAL_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxpkcllrxxxxxxxxx003652646977xxxxxxxxxpkcllr",
        );
        check_address(
            GLOBAL_CALLER_VIRTUAL_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxglcllrxxxxxxxxx002350006550xxxxxxxxxglcllr",
        );

        // Transaction badges
        check_address(
            SYSTEM_TRANSACTION_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxsystxnxxxxxxxxx002683325037xxxxxxxxxsystxn",
        );

        // Entity owner badges
        check_address(
            PACKAGE_OWNER_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxpkgwnrxxxxxxxxx002558553505xxxxxxxxxpkgwnr",
        );
        check_address(
            VALIDATOR_OWNER_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxvdrwnrxxxxxxxxx004365253834xxxxxxxxxvdrwnr",
        );
        check_address(
            ACCOUNT_OWNER_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxaccwnrxxxxxxxxx006664022062xxxxxxxxxaccwnr",
        );
        check_address(
            IDENTITY_OWNER_BADGE.as_ref(),
            EntityType::GlobalNonFungibleResourceManager,
            "resource_rdx1nfxxxxxxxxxxdntwnrxxxxxxxxx002876444928xxxxxxxxxdntwnr",
        );

        // Packages
        check_address(
            PACKAGE_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxpackgexxxxxxxxx000726633226xxxxxxxxxpackge",
        );
        check_address(
            RESOURCE_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxresrcexxxxxxxxx000538436477xxxxxxxxxresrce",
        );
        check_address(
            ACCOUNT_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxaccntxxxxxxxxxx000929625493xxxxxxxxxaccntx",
        );
        check_address(
            IDENTITY_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxdntyxxxxxxxxxxx008560783089xxxxxxxxxdntyxx",
        );
        check_address(
            CONSENSUS_MANAGER_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxcnsmgrxxxxxxxxx000746305335xxxxxxxxxcnsmgr",
        );
        check_address(
            ACCESS_CONTROLLER_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxcntrlrxxxxxxxxx000648572295xxxxxxxxxcntrlr",
        );
        check_address(
            POOL_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxplxxxxxxxxxxxxx020379220524xxxxxxxxxplxxxx",
        );
        check_address(
            TRANSACTION_PROCESSOR_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxtxnpxrxxxxxxxxx002962227406xxxxxxxxxtxnpxr",
        );
        check_address(
            METADATA_MODULE_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxmtdataxxxxxxxxx005246577269xxxxxxxxxmtdata",
        );
        check_address(
            ROYALTY_MODULE_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxryaltyxxxxxxxxx003849573396xxxxxxxxxryalty",
        );
        check_address(
            ROLE_ASSIGNMENT_MODULE_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxarulesxxxxxxxxx002304462983xxxxxxxxxarules",
        );
        check_address(
            GENESIS_HELPER_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxgenssxxxxxxxxxx004372642773xxxxxxxxxgenssx",
        );
        check_address(
            FAUCET_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxfaucetxxxxxxxxx000034355863xxxxxxxxxfaucet",
        );
        check_address(
            TRANSACTION_TRACKER_PACKAGE.as_ref(),
            EntityType::GlobalPackage,
            "package_rdx1pkgxxxxxxxxxtxtrakxxxxxxxxx000595975309xxxxxxxxxtxtrak",
        );

        // System singleton components - native
        check_address(
            CONSENSUS_MANAGER.as_ref(),
            EntityType::GlobalConsensusManager,
            "consensusmanager_rdx1scxxxxxxxxxxcnsmgrxxxxxxxxx000999665565xxxxxxxxxcnsmgr",
        );

        // System singleton components - scrypto
        check_address(
            FAUCET.as_ref(),
            EntityType::GlobalGenericComponent,
            "component_rdx1cptxxxxxxxxxfaucetxxxxxxxxx000527798379xxxxxxxxxfaucet",
        );
        check_address(
            GENESIS_HELPER.as_ref(),
            EntityType::GlobalGenericComponent,
            "component_rdx1cptxxxxxxxxxgenssxxxxxxxxxx000977302539xxxxxxxxxgenssx",
        );
        check_address(
            TRANSACTION_TRACKER.as_ref(),
            EntityType::GlobalTransactionTracker,
            "transactiontracker_rdx1stxxxxxxxxxxtxtrakxxxxxxxxx006844685494xxxxxxxxxtxtrak",
        );
    }

    fn check_address(address_bytes: &[u8], entity_type: EntityType, address_string: &str) {
        assert_eq!(address_bytes[0], entity_type as u8);
        let encoded_address = AddressBech32Encoder::new(&NetworkDefinition::mainnet())
            .encode(address_bytes)
            .unwrap();
        assert_eq!(encoded_address.as_str(), address_string);
    }
}
