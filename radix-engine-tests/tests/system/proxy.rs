use radix_engine_common::prelude::*;
use radix_engine_tests::common::*;
use scrypto_test::prelude::*;

fn initialize_package(
    test_runner: &mut DefaultTestRunner,
    owner_role: Option<OwnerRole>,
    package_name: &str,
    blueprint_name: &str,
    function_name: &str,
) -> ComponentAddress {
    let package_address = test_runner.publish_package_simple(PackageLoader::get(package_name));

    let args = if let Some(owner_role) = owner_role {
        manifest_args!(owner_role)
    } else {
        manifest_args!()
    };

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package_address, blueprint_name, function_name, args)
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let my_component = receipt.expect_commit(true).new_component_addresses()[0];
    my_component
}

fn create_some_resources(test_runner: &mut DefaultTestRunner) -> IndexMap<String, ResourceAddress> {
    let (_public_key, _, account_address) = test_runner.new_account(false);
    let mut resources = indexmap!();

    for symbol in ["XRD", "USDT", "ETH"] {
        resources.insert(
            symbol.to_string(),
            test_runner.create_fungible_resource(dec!(20000), 18, account_address),
        );
    }
    resources
}

fn oracle_configure<T>(
    test_runner: &mut DefaultTestRunner,
    proofs: T,
    resources: &IndexMap<String, ResourceAddress>,
    oracle_component_address: ComponentAddress,
) where
    T: IntoIterator<Item = NonFungibleGlobalId> + Clone,
{
    // "set_price" is a protected method, need to be called directly on the Oracle component
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            oracle_component_address,
            "set_price",
            manifest_args!(
                resources.get("XRD").unwrap(),
                resources.get("USDT").unwrap(),
                dec!(20)
            ),
        )
        .call_method(
            oracle_component_address,
            "set_price",
            manifest_args!(
                resources.get("XRD").unwrap(),
                resources.get("ETH").unwrap(),
                dec!(30)
            ),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, proofs.clone());
    receipt.expect_commit_success();
}

fn oracle_v3_configure<T>(
    test_runner: &mut DefaultTestRunner,
    proofs: T,
    resources: &IndexMap<String, ResourceAddress>,
    oracle_component_address: ComponentAddress,
) where
    T: IntoIterator<Item = NonFungibleGlobalId> + Clone,
{
    // "set_price" and "add_symbol" are protected methods, need to be called directly on the Oracle component
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            oracle_component_address,
            "add_symbol",
            manifest_args!(resources.get("XRD").unwrap(), "XRD".to_string()),
        )
        .call_method(
            oracle_component_address,
            "add_symbol",
            manifest_args!(resources.get("USDT").unwrap(), "USDT".to_string()),
        )
        .call_method(
            oracle_component_address,
            "add_symbol",
            manifest_args!(resources.get("ETH").unwrap(), "ETH".to_string()),
        )
        .call_method(
            oracle_component_address,
            "set_price",
            manifest_args!("XRD".to_string(), "USDT".to_string(), dec!(20)),
        )
        .call_method(
            oracle_component_address,
            "set_price",
            manifest_args!("XRD".to_string(), "ETH".to_string(), dec!(30)),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, proofs);
    receipt.expect_commit_success();
}

fn invoke_oracle_via_proxy_basic<T>(
    test_runner: &mut DefaultTestRunner,
    proofs: T,
    resources: &IndexMap<String, ResourceAddress>,
    proxy_component_address: ComponentAddress,
    oracle_component_address: ComponentAddress,
    info: &str,
) where
    T: IntoIterator<Item = NonFungibleGlobalId> + Clone,
{
    // Set Oracle component address in Proxy
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "set_component_address",
            manifest_args!(oracle_component_address),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, proofs.clone());
    receipt.expect_commit_success();

    oracle_configure(
        test_runner,
        proofs.clone(),
        resources,
        oracle_component_address,
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "proxy_get_price",
            manifest_args!(
                resources.get("XRD").unwrap(),
                resources.get("USDT").unwrap(),
            ),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let price: Option<Decimal> = receipt.expect_commit_success().output(1);

    // Assert
    assert_eq!(price.unwrap(), dec!(20));

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "proxy_get_oracle_info",
            manifest_args!(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let oracle_info: String = receipt.expect_commit_success().output(1);

    // Assert
    assert_eq!(&oracle_info, info);
}

fn invoke_oracle_via_generic_proxy<T>(
    test_runner: &mut DefaultTestRunner,
    proofs: T,
    resources: &IndexMap<String, ResourceAddress>,
    proxy_component_address: ComponentAddress,
    oracle_component_address: ComponentAddress,
    info: &str,
) where
    T: IntoIterator<Item = NonFungibleGlobalId> + Clone,
{
    // Set Oracle component address in Proxy
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "set_component_address",
            manifest_args!(oracle_component_address),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, proofs.clone());
    receipt.expect_commit_success();

    oracle_configure(
        test_runner,
        proofs.clone(),
        resources,
        oracle_component_address,
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "proxy_call",
            manifest_args!(
                "get_price",
                to_manifest_value(&(
                    resources.get("XRD").unwrap(),
                    resources.get("USDT").unwrap(),
                ))
                .unwrap()
            ),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let price: Option<Decimal> = receipt.expect_commit_success().output(1);

    // Assert
    assert_eq!(price.unwrap(), dec!(20));

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "proxy_call",
            manifest_args!("get_oracle_info", to_manifest_value(&()).unwrap()),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let oracle_info: String = receipt.expect_commit_success().output(1);

    // Assert
    assert_eq!(&oracle_info, info);
}

fn invoke_oracle_v3_via_generic_proxy<T>(
    test_runner: &mut DefaultTestRunner,
    proofs: T,
    resources: &IndexMap<String, ResourceAddress>,
    proxy_component_address: ComponentAddress,
    oracle_component_address: ComponentAddress,
    info: &str,
) where
    T: IntoIterator<Item = NonFungibleGlobalId> + Clone,
{
    // Set Oracle component address in Proxy
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "set_component_address",
            manifest_args!(oracle_component_address),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, proofs.clone());
    receipt.expect_commit_success();

    oracle_v3_configure(
        test_runner,
        proofs.clone(),
        resources,
        oracle_component_address,
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "proxy_call",
            manifest_args!(
                "get_address",
                // Note the comma in below tuple reference &(,)
                // Function arguments must be encoded to ManifestValue as a tuple, even if it is
                // just a single argument.
                // Without comma a single argument is encoded with it's native type omitting the
                // tuple.
                to_manifest_value(&("ETH".to_string(),)).unwrap()
            ),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let eth_resource_address: Option<ResourceAddress> = receipt.expect_commit_success().output(1);

    // Assert
    assert_eq!(
        &eth_resource_address.unwrap(),
        resources.get("ETH").unwrap()
    );

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "proxy_call",
            manifest_args!(
                "get_price",
                to_manifest_value(&("XRD".to_string(), "USDT".to_string())).unwrap()
            ),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let price: Option<Decimal> = receipt.expect_commit_success().output(1);

    // Assert
    assert_eq!(price.unwrap(), dec!(20));

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            proxy_component_address,
            "proxy_call",
            manifest_args!("get_oracle_info", to_manifest_value(&()).unwrap()),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let oracle_info: String = receipt.expect_commit_success().output(1);

    // Assert
    assert_eq!(&oracle_info, info);
}

#[test]
fn test_proxy_basic() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let resources = create_some_resources(&mut test_runner);
    let (public_key, _, _account) = test_runner.new_account(false);
    let owner_badge = NonFungibleGlobalId::from_public_key(&public_key);

    let rule = rule!(require(owner_badge.clone()));
    let owner_role = OwnerRole::Fixed(rule);

    // Publish and instantiate Oracle Proxy
    let proxy_component_address = initialize_package(
        &mut test_runner,
        Some(owner_role.clone()),
        "oracle_proxy_basic",
        "OracleProxy",
        "instantiate_proxy",
    );

    // Publish and instantiate Oracle v1
    let oracle_v1_component_address = initialize_package(
        &mut test_runner,
        Some(owner_role.clone()),
        "oracle_v1",
        "Oracle",
        "instantiate_global",
    );

    // Perform some operations on Oracle v1
    invoke_oracle_via_proxy_basic(
        &mut test_runner,
        vec![owner_badge.clone()],
        &resources,
        proxy_component_address,
        oracle_v1_component_address,
        "Oracle v1",
    );

    // Publish and instantiate Oracle v2
    let oracle_v2_component_address = initialize_package(
        &mut test_runner,
        Some(owner_role),
        "oracle_v2",
        "Oracle",
        "instantiate_global",
    );

    // Perform some operations on Oracle v2
    invoke_oracle_via_proxy_basic(
        &mut test_runner,
        vec![owner_badge.clone()],
        &resources,
        proxy_component_address,
        oracle_v2_component_address,
        "Oracle v2",
    );
}

#[test]
fn test_proxy_generic() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let resources = create_some_resources(&mut test_runner);
    let (public_key, _, _account) = test_runner.new_account(false);
    let owner_badge = NonFungibleGlobalId::from_public_key(&public_key);

    let rule = rule!(require(owner_badge.clone()));
    let owner_role = OwnerRole::Fixed(rule);

    // Publish and instantiate Oracle Proxy
    let proxy_component_address = initialize_package(
        &mut test_runner,
        Some(owner_role.clone()),
        "generic_proxy",
        "GenericProxy",
        "instantiate_proxy",
    );

    // Publish and instantiate Oracle v1
    let oracle_v1_component_address = initialize_package(
        &mut test_runner,
        Some(owner_role.clone()),
        "oracle_v1",
        "Oracle",
        "instantiate_global",
    );

    // Perform some operations on Oracle v1
    invoke_oracle_via_generic_proxy(
        &mut test_runner,
        vec![owner_badge.clone()],
        &resources,
        proxy_component_address,
        oracle_v1_component_address,
        "Oracle v1",
    );

    // Publish and instantiate Oracle v2
    let oracle_v2_component_address = initialize_package(
        &mut test_runner,
        Some(owner_role.clone()),
        "oracle_v2",
        "Oracle",
        "instantiate_global",
    );

    // Perform some operations on Oracle v2
    invoke_oracle_via_generic_proxy(
        &mut test_runner,
        vec![owner_badge.clone()],
        &resources,
        proxy_component_address,
        oracle_v2_component_address,
        "Oracle v2",
    );

    // Publish and instantiate Oracle v3
    let oracle_v3_component_address = initialize_package(
        &mut test_runner,
        Some(owner_role.clone()),
        "oracle_v3",
        "Oracle",
        "instantiate_global",
    );

    // Perform some operations on Oracle v3
    // Note that Oracle v3 has different API than v1 and v2
    invoke_oracle_v3_via_generic_proxy(
        &mut test_runner,
        vec![owner_badge.clone()],
        &resources,
        proxy_component_address,
        oracle_v3_component_address,
        "Oracle v3",
    );
}
