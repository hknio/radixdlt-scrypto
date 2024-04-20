use std::path::PathBuf;

use radix_engine_interface::{blueprints::resource::OwnerRole, metadata_init};
use radix_transactions::builder::ManifestBuilder;
use scrypto_test::ledger_simulator::LedgerSimulatorBuilder;
use scrypto_wasm_fuzzer::*;

pub fn main() {
    // Arrange
    let mut ledger = LedgerSimulatorBuilder::new().without_kernel_trace().build();

    let (code, definition) = build_with_coverage(PathBuf::from("/workspaces/develop/scrypto-wasm-fuzzer/fuzz_blueprint"));
    //let code = include_bytes!("fuzz_blueprint.wasm").to_vec();
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(None, code, definition, metadata_init!(), OwnerRole::None)
        .build();
    let receipt = ledger.execute_manifest(manifest, vec![]);
    let package_address = receipt.expect_commit(true).new_package_addresses()[0];

    println!("package_address: {:?}", package_address);

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package_address, "FuzzBlueprint", "fuzz", (Vec::<u8>::new(), ))
        .build();
    let receipt = ledger.execute_manifest(manifest, vec![]);
    receipt.expect_commit_success();

    /*
    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            CONSENSUS_MANAGER,
            CONSENSUS_MANAGER_NEXT_ROUND_IDENT,
            ConsensusManagerNextRoundInput::successful(Round::of(1), 0, time_to_set_ms),
        )
        .call_function(
            package_address,
            "ClockTest",
            "get_current_time_rounded_to_minutes",
            manifest_args![],
        )
        .build();
    let receipt = ledger.execute_manifest(manifest, vec![AuthAddresses::validator_role()]);

    // Assert
    let current_unix_time_rounded_to_minutes: i64 = receipt.expect_commit(true).output(2);
    assert_eq!(
        current_unix_time_rounded_to_minutes,
        expected_unix_time_rounded_to_minutes
    );

     */
    println!("works");
}
