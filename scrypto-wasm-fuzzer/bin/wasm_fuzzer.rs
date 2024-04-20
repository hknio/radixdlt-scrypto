#![no_main]

use std::path::PathBuf;

use radix_common::data::scrypto;
use radix_engine_interface::{blueprints::resource::OwnerRole, metadata_init};
use radix_transactions::builder::ManifestBuilder;
use scrypto_test::ledger_simulator::LedgerSimulatorBuilder;
use scrypto_wasm_fuzzer::*;
use scrypto_test::prelude::*;

extern crate libc;

use libc::c_uint;
use libc::size_t;

extern "C" {
    fn __sanitizer_cov_8bit_counters_init(start: *mut u8, end: *mut u8);
}

static mut COUNTERS : Option<Vec<u8>> = None;
static mut LEDGER : Option<LedgerSimulator<NoExtension, InMemorySubstateDatabase>> = None;
static mut PACKAGE_ADDRESS : Option<PackageAddress> = None;

#[no_mangle]
pub extern "C" fn LLVMFuzzerInitialize(_argc: *const c_uint, _argv: *const *const *const u8) -> c_uint {
    let mut ledger = LedgerSimulatorBuilder::new().without_kernel_trace().build();
    let (code, definition) = build_with_coverage(PathBuf::from("/workspaces/develop/scrypto-wasm-fuzzer/fuzz_blueprint"));
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .publish_package_advanced(None, code, definition, metadata_init!(), OwnerRole::None)
        .build();
    let receipt = ledger.execute_manifest(manifest, vec![]);
    let package_address = receipt.expect_commit(true).new_package_addresses()[0];
    println!("package_address: {:?}", package_address);
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package_address, "FuzzBlueprint", "get_counters_size", ())
        .build();
    let receipt = ledger.execute_manifest(manifest, vec![]);
    let counters_len : usize = receipt.expect_commit_success().output(1);
    println!("counters_len: {:?}", counters_len);

    unsafe {
        COUNTERS = Some(vec![0; counters_len]);
        LEDGER = Some(ledger);
        PACKAGE_ADDRESS = Some(package_address);

        let start_ptr = COUNTERS.as_mut().unwrap().as_mut_ptr();
        let end_ptr = start_ptr.add(counters_len);
        __sanitizer_cov_8bit_counters_init(start_ptr, end_ptr);
    }
    0
}

#[no_mangle]
pub extern "C" fn LLVMFuzzerTestOneInput(data: *const u8, size: size_t) -> c_uint {
    let slice = unsafe {
        // Convert the raw pointer to a slice for safer access. This is still unsafe because we are
        // trusting the fuzzer to provide valid data and size.
        std::slice::from_raw_parts(data, size)
    };

    let data = slice.to_vec();

    let counters = unsafe {
        let manifest = ManifestBuilder::new()
            .call_function(PACKAGE_ADDRESS.unwrap(), "FuzzBlueprint", "fuzz", (data, ))
            .build();
        let receipt = LEDGER.as_mut().unwrap().preview_manifest(
            manifest,
            Default::default(),
    Default::default(),
            PreviewFlags {
                use_free_credit: true,
                assume_all_signature_proofs: true,
                skip_epoch_check: true,
                disable_auth: false,
        });

        let counters : Vec<u8> = receipt.expect_commit_success().output(0);
        counters
    };

    unsafe { 
        COUNTERS.as_mut().unwrap().iter_mut().zip(counters.iter()).for_each(|(a, b)| *a += b);
    }

    // Here you would add the actual fuzzing logic, e.g., processing the input data.
    // For example:
    //if size > 0 && unsafe { *data } == b'G' {
        //panic!("The first byte is 'G'!");
    //}



    // Return 0 to indicate to the fuzzer that everything is okay.
    0
}
/*

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
 */