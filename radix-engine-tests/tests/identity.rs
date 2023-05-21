use radix_engine::errors::{ModuleError, RuntimeError};
use radix_engine::system::system_modules::auth::AuthError;
use radix_engine::types::*;
use radix_engine_interface::api::node_modules::metadata::MetadataValue;
use radix_engine_interface::blueprints::account::ACCOUNT_TRY_DEPOSIT_BATCH_UNSAFE_IDENT;
use radix_engine_interface::blueprints::identity::{
    IdentitySecurifyToSingleBadgeInput, IDENTITY_SECURIFY_IDENT,
};
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;
use transaction::model::Instruction;

#[test]
fn cannot_securify_in_advanced_mode() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (pk, _, account) = test_runner.new_account(false);
    let component_address = test_runner.new_identity(pk.clone(), false);

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .add_instruction(Instruction::CallMethod {
            address: component_address.into(),
            method_name: IDENTITY_SECURIFY_IDENT.to_string(),
            args: to_manifest_value(&IdentitySecurifyToSingleBadgeInput {}),
        })
        .0
        .call_method(
            account,
            ACCOUNT_TRY_DEPOSIT_BATCH_UNSAFE_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt =
        test_runner.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&pk)]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ModuleError(ModuleError::AuthError(AuthError::Unauthorized { .. }))
        )
    });
}

#[test]
fn can_securify_from_virtual_identity() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (pk, _, account) = test_runner.new_account(false);
    let component_address = test_runner.new_identity(pk.clone(), true);

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .add_instruction(Instruction::CallMethod {
            address: component_address.into(),
            method_name: IDENTITY_SECURIFY_IDENT.to_string(),
            args: to_manifest_value(&IdentitySecurifyToSingleBadgeInput {}),
        })
        .0
        .call_method(
            account,
            ACCOUNT_TRY_DEPOSIT_BATCH_UNSAFE_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt =
        test_runner.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&pk)]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn cannot_securify_twice() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (pk, _, account) = test_runner.new_account(false);
    let component_address = test_runner.new_identity(pk.clone(), true);
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .add_instruction(Instruction::CallMethod {
            address: component_address.into(),
            method_name: IDENTITY_SECURIFY_IDENT.to_string(),
            args: to_manifest_value(&IdentitySecurifyToSingleBadgeInput {}),
        })
        .0
        .call_method(
            account,
            ACCOUNT_TRY_DEPOSIT_BATCH_UNSAFE_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt =
        test_runner.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&pk)]);
    receipt.expect_commit_success();

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .add_instruction(Instruction::CallMethod {
            address: component_address.into(),
            method_name: IDENTITY_SECURIFY_IDENT.to_string(),
            args: to_manifest_value(&IdentitySecurifyToSingleBadgeInput {}),
        })
        .0
        .call_method(
            account,
            ACCOUNT_TRY_DEPOSIT_BATCH_UNSAFE_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt =
        test_runner.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&pk)]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::ModuleError(ModuleError::AuthError(AuthError::Unauthorized { .. }))
        )
    });
}

#[test]
fn can_set_metadata_after_securify() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (pk, _, account) = test_runner.new_account(false);
    let identity_address = test_runner.new_identity(pk.clone(), true);
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .add_instruction(Instruction::CallMethod {
            address: identity_address.into(),
            method_name: IDENTITY_SECURIFY_IDENT.to_string(),
            args: to_manifest_value(&IdentitySecurifyToSingleBadgeInput {}),
        })
        .0
        .call_method(
            account,
            ACCOUNT_TRY_DEPOSIT_BATCH_UNSAFE_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt =
        test_runner.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&pk)]);
    receipt.expect_commit_success();

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .create_proof_from_account(account, IDENTITY_OWNER_BADGE)
        .set_metadata(
            identity_address.into(),
            "name".to_string(),
            MetadataValue::String("best package ever!".to_string()),
        )
        .build();
    let receipt =
        test_runner.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&pk)]);

    // Assert
    receipt.expect_commit_success();
    let value = test_runner
        .get_metadata(identity_address.into(), "name")
        .expect("Should exist");
    assert_eq!(
        value,
        MetadataValue::String("best package ever!".to_string())
    );
}

#[test]
fn can_set_metadata_on_securified_identity() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let (pk, _, account) = test_runner.new_account(false);
    let identity_address = test_runner.new_securified_identity(account);

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(test_runner.faucet_component(), 10.into())
        .create_proof_from_account(account, IDENTITY_OWNER_BADGE)
        .set_metadata(
            identity_address.into(),
            "name".to_string(),
            MetadataValue::String("best package ever!".to_string()),
        )
        .build();
    let receipt =
        test_runner.execute_manifest(manifest, vec![NonFungibleGlobalId::from_public_key(&pk)]);

    // Assert
    receipt.expect_commit_success();
    let value = test_runner
        .get_metadata(identity_address.into(), "name")
        .expect("Should exist");
    assert_eq!(
        value,
        MetadataValue::String("best package ever!".to_string())
    );
}
