use radix_engine::errors::{RuntimeError, SystemError};
use radix_engine::types::*;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;

#[test]
fn should_not_be_able_to_node_create_with_invalid_blueprint() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/kernel");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 500u32.into())
        .call_function(
            package_address,
            "NodeCreate",
            "create_node_with_invalid_blueprint",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| match e {
        RuntimeError::SystemError(SystemError::BlueprintDoesNotExist(..)) => true,
        _ => false,
    });
}
