use scrypto::prelude::*;

#[blueprint]
mod child_component {
    struct ChildComponent {
        to_call: ComponentAddress,
    }

    impl ChildComponent {
        pub fn create(to_call: ComponentAddress) -> ChildComponentComponent {
            Self { to_call }.instantiate()
        }

        pub fn get_global_address(&self) -> ComponentAddress {
            Runtime::global_address()
        }

        pub fn call_other_component(&self, child: bool) {
            let _: () = Runtime::call_method(
                self.to_call,
                "protected_method",
                scrypto_args!(Runtime::global_address(), child),
            );
        }

        pub fn assert_check_on_package(&self, package_address: PackageAddress) {
            Runtime::assert_access_rule(rule!(require(package_of_caller(package_address))));
        }
    }
}

#[blueprint]
mod my_component {
    use child_component::*;

    struct MyComponent {
        child: ChildComponentComponent,
        to_call: ComponentAddress,
    }

    impl MyComponent {
        pub fn create(to_call: ComponentAddress) -> ComponentAddress {
            let child = ChildComponent::create(to_call);
            Self { child, to_call }.instantiate().globalize()
        }

        pub fn get_global_address_in_parent(&self) -> ComponentAddress {
            Runtime::global_address()
        }

        pub fn get_global_address_in_owned(&self) -> ComponentAddress {
            self.child.get_global_address()
        }

        pub fn get_global_address_in_local(to_call: ComponentAddress) -> ComponentAddress {
            let child = ChildComponent::create(to_call);
            let address = child.get_global_address();
            Self { child, to_call }.instantiate().globalize();
            address
        }

        pub fn get_global_address_in_local_of_parent_method(
            &self,
            to_call: ComponentAddress,
        ) -> ComponentAddress {
            Self::get_global_address_in_local(to_call)
        }

        pub fn call_other_component_with_wrong_address(&self) {
            let address = self.to_call;
            Runtime::call_method(
                self.to_call,
                "protected_method",
                scrypto_args!(address, false),
            )
        }

        pub fn call_other_component(&self, child: bool, called_child: bool) {
            if child {
                self.child.call_other_component(called_child);
            } else {
                Runtime::call_method(
                    self.to_call,
                    "protected_method",
                    scrypto_args!(Runtime::global_address(), called_child),
                )
            }
        }

        pub fn assert_check_on_package(&self, package_address: PackageAddress, child: bool) {
            if child {
                self.child.assert_check_on_package(package_address);
            } else {
                Runtime::assert_access_rule(rule!(require(package_of_caller(package_address))));
            }
        }
    }
}

#[blueprint]
mod called_component {
    use called_component_child::*;

    struct CalledComponent {
        child: CalledComponentChildComponent,
    }

    impl CalledComponent {
        pub fn create() -> ComponentAddress {
            let child = CalledComponentChild::create();
            Self { child }.instantiate().globalize()
        }

        pub fn protected_method(&self, component_address: ComponentAddress, child: bool) {
            if child {
                self.child.protected_method(component_address);
            } else {
                Runtime::assert_access_rule(rule!(require(global_caller(component_address))));
                assert_ne!(Runtime::global_address(), component_address.into());
            }
        }
    }
}

#[blueprint]
mod called_component_child {
    struct CalledComponentChild {}

    impl CalledComponentChild {
        pub fn create() -> CalledComponentChildComponent {
            Self {}.instantiate()
        }

        pub fn protected_method(&self, component_address: ComponentAddress) {
            Runtime::assert_access_rule(rule!(require(global_caller(component_address))));
            assert_ne!(Runtime::global_address(), component_address.into());
        }
    }
}

#[blueprint]
mod preallocation_component {
    struct PreallocationComponent {}

    impl PreallocationComponent {
        pub fn create_with_preallocated_address() -> ComponentAddress {
            let component_address = Runtime::preallocate_global_component_address();
            Self {}
                .instantiate()
                .globalize_at_address(component_address)
        }

        pub fn create_with_unused_preallocated_address_1() -> ComponentAddress {
            let component_address = Runtime::preallocate_global_component_address();
            Runtime::preallocate_global_component_address();
            Self {}
                .instantiate()
                .globalize_at_address(component_address)
        }

        pub fn create_with_unused_preallocated_address_2() -> ComponentAddress {
            Runtime::preallocate_global_component_address();
            Self {}.instantiate().globalize()
        }

        pub fn create_two_with_same_address() -> (ComponentAddress, ComponentAddress) {
            let component_address = Runtime::preallocate_global_component_address();
            let one = Self {}
                .instantiate()
                .globalize_at_address(component_address);
            let two = Self {}
                .instantiate()
                .globalize_at_address(component_address);
            (one, two)
        }
    }
}

#[blueprint]
mod preallocation_smuggler_component {
    struct PreallocationSmugglerComponent {
        preallocated_address: Option<GlobalAddress>,
    }

    impl PreallocationSmugglerComponent {
        pub fn create_empty() -> ComponentAddress {
            Self {
                preallocated_address: None,
            }
            .instantiate()
            .globalize()
        }

        pub fn create_empty_at_address_bytes(
            preallocated_address_bytes: [u8; 30],
        ) -> ComponentAddress {
            let component_address =
                unsafe { ComponentAddress::new_unchecked(preallocated_address_bytes) };
            Self {
                preallocated_address: None,
            }
            .instantiate()
            .globalize_at_address(component_address)
        }

        pub fn create_empty_at_address(preallocated_address: ComponentAddress) -> ComponentAddress {
            Self {
                preallocated_address: None,
            }
            .instantiate()
            .globalize_at_address(preallocated_address)
        }

        pub fn create_with_smuggled_address() -> ComponentAddress {
            Self {
                preallocated_address: Some(Runtime::preallocate_global_component_address().into()),
            }
            .instantiate()
            .globalize()
        }

        pub fn create_with_smuggled_given_address(address: GlobalAddress) -> ComponentAddress {
            Self {
                preallocated_address: Some(address),
            }
            .instantiate()
            .globalize()
        }

        pub fn create_with_smuggled_given_address_bytes(
            preallocated_address_bytes: [u8; 30],
        ) -> ComponentAddress {
            let address = unsafe { GlobalAddress::new_unchecked(preallocated_address_bytes) };
            Self {
                preallocated_address: Some(address),
            }
            .instantiate()
            .globalize()
        }

        pub fn smuggle_given_address(&mut self, address: GlobalAddress) {
            self.preallocated_address = Some(address);
        }

        pub fn allocate_and_smuggle_address(&mut self) {
            self.preallocated_address =
                Some(Runtime::preallocate_global_component_address().into());
        }

        pub fn instantiate_with_smuggled_address(&self) -> ComponentAddress {
            let component_address = unsafe {
                ComponentAddress::new_unchecked(self.preallocated_address.unwrap().as_node_id().0)
            };
            Self {
                preallocated_address: None,
            }
            .instantiate()
            .globalize_at_address(component_address)
        }
    }
}
