use criterion::{criterion_group, criterion_main, Criterion};
use paste::paste;
use radix_engine::{
    system::system_modules::costing::SystemLoanFeeReserve,
    transaction::CostingParameters,
    types::*,
    utils::ExtractSchemaError,
    vm::{
        wasm::{
            DefaultWasmEngine, WasmEngine, WasmInstance, WasmModule, WasmRuntime, WasmValidator,
        },
        wasm_runtime::NoOpWasmRuntime,
    },
};
use radix_engine_queries::typed_substate_layout::PackageDefinition;
use sbor::rust::iter;
use scrypto_unit::TestRunnerBuilder;
use transaction::{
    prelude::{Secp256k1PrivateKey, TransactionCostingParameters},
    validation::{recover_secp256k1, verify_secp256k1},
};
use wabt::wat2wasm;

fn bench_decode_sbor(c: &mut Criterion) {
    let payload = include_bytes!("../../assets/radiswap.rpd");
    println!("Payload size: {}", payload.len());
    c.bench_function("costing::decode_sbor", |b| {
        b.iter(|| manifest_decode::<ManifestValue>(payload))
    });
}

fn bench_validate_sbor_payload(c: &mut Criterion) {
    let package_definition =
        manifest_decode::<PackageDefinition>(include_bytes!("../../assets/radiswap.rpd")).unwrap();
    let payload = scrypto_encode(&package_definition).unwrap();
    println!("Payload size: {}", payload.len());
    let (index, schema) =
        generate_full_schema_from_single_type::<PackageDefinition, ScryptoCustomSchema>();

    c.bench_function("costing::validate_sbor_payload", |b| {
        b.iter(|| {
            validate_payload_against_schema::<ScryptoCustomExtension, _>(
                &payload,
                &schema,
                index,
                &(),
            )
        })
    });
}

fn bench_validate_secp256k1(c: &mut Criterion) {
    let message = "m".repeat(1_000_000);
    let message_hash = hash(message.as_bytes());
    let signer = Secp256k1PrivateKey::from_u64(123123123123).unwrap();
    let signature = signer.sign(&message_hash);

    c.bench_function("costing::validate_secp256k1", |b| {
        b.iter(|| {
            let public_key = recover_secp256k1(&message_hash, &signature).unwrap();
            verify_secp256k1(&message_hash, &public_key, &signature);
        })
    });
}

fn bench_spin_loop(c: &mut Criterion) {
    // Prepare code
    let code = wat2wasm(&include_str!("../tests/wasm/loop.wat").replace("${n}", "100000")).unwrap();

    // Instrument
    let validator = WasmValidator::default();
    let instrumented_code = validator
        .validate(&code, iter::empty())
        .map_err(|e| ExtractSchemaError::InvalidWasm(e))
        .unwrap()
        .0;

    // Note that wasm engine maintains an internal cache, which means costing
    // isn't taking WASM parsing into consideration.
    let wasm_engine = DefaultWasmEngine::default();
    let mut wasm_execution_units_consumed = 0;
    c.bench_function("costing::spin_loop", |b| {
        b.iter(|| {
            let fee_reserve = SystemLoanFeeReserve::new(
                &CostingParameters::default(),
                &TransactionCostingParameters {
                    free_credit_in_xrd: Decimal::try_from(FREE_CREDIT_IN_XRD).unwrap(),
                    tip_percentage: DEFAULT_TIP_PERCENTAGE,
                },
                false,
            );
            wasm_execution_units_consumed = 0;
            let mut runtime: Box<dyn WasmRuntime> = Box::new(NoOpWasmRuntime::new(
                fee_reserve,
                &mut wasm_execution_units_consumed,
            ));
            let mut instance = wasm_engine.instantiate(Hash([0u8; 32]), &instrumented_code);
            instance
                .invoke_export("Test_f", vec![Buffer(0)], &mut runtime)
                .unwrap();
        })
    });

    println!(
        "WASM execution units consumed: {}",
        wasm_execution_units_consumed
    );
}

macro_rules! bench_instantiate {
    ($what:literal) => {
        paste! {
        fn [< bench_instantiate_ $what >] (c: &mut Criterion) {
            // Prepare code
            let code = include_bytes!(concat!("../../assets/", $what, ".wasm"));

            // Instrument
            let validator = WasmValidator::default();
            let instrumented_code = validator
                .validate(code, iter::empty())
                .map_err(|e| ExtractSchemaError::InvalidWasm(e))
                .unwrap()
                .0;

            c.bench_function(concat!("costing::instantiate_", $what), |b| {
                b.iter(|| {
                    let wasm_engine = DefaultWasmEngine::default();
                    wasm_engine.instantiate(Hash([0u8; 32]), &instrumented_code);
                })
            });

            println!("Code length: {}", instrumented_code.len());
        }
        }
    };
}

bench_instantiate!("radiswap");
bench_instantiate!("flash_loan");

fn bench_validate_wasm(c: &mut Criterion) {
    let code = include_bytes!("../../assets/radiswap.wasm");
    let definition: PackageDefinition =
        manifest_decode(include_bytes!("../../assets/radiswap.rpd")).unwrap();

    c.bench_function("costing::validate_wasm", |b| {
        b.iter(|| {
            WasmValidator::default()
                .validate(code, definition.blueprints.values())
                .unwrap()
        })
    });

    println!("Code length: {}", code.len());
}

fn bench_deserialize_wasm(c: &mut Criterion) {
    let code = include_bytes!("../../assets/radiswap.wasm");

    c.bench_function("costing::deserialize_wasm", |b| {
        b.iter(|| WasmModule::init(code).unwrap())
    });
}

fn bench_prepare_wasm(c: &mut Criterion) {
    let mut test_runner = TestRunnerBuilder::new().without_trace().build();
    let code = include_bytes!("../../assets/radiswap.wasm").to_vec();
    let package_definition: PackageDefinition =
        manifest_decode(include_bytes!("../../assets/radiswap.rpd")).unwrap();

    c.bench_function("costing::bench_prepare_wasm", |b| {
        b.iter(|| {
            let (pk1, _, _) = test_runner.new_allocated_account();
            test_runner.publish_package(
                code.clone(),
                package_definition.clone(),
                btreemap!(),
                OwnerRole::Updatable(rule!(require(NonFungibleGlobalId::from_public_key(&pk1)))),
            );
        })
    });
}

criterion_group!(
    costing,
    bench_decode_sbor,
    bench_validate_sbor_payload,
    bench_validate_secp256k1,
    bench_spin_loop,
    bench_instantiate_radiswap,
    bench_instantiate_flash_loan,
    bench_deserialize_wasm,
    bench_validate_wasm,
    bench_prepare_wasm,
);
criterion_main!(costing);
