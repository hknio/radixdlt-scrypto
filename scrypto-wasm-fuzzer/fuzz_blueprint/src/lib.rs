use scrypto::prelude::*;

#[blueprint]
mod fuzz_blueprint {
    struct FuzzBlueprint;

    impl FuzzBlueprint {
        pub fn fuzz(input: Vec<u8>) {
            let map = scrypto_decode::<IndexMap<u32, IndexSet<(String, Decimal)>>>(&input);
            if map.is_ok() {
                let map = map.unwrap();;
                if map.len() > 0 {
                    if map.contains_key(&123456) {
                        panic!("wow");
                    }
                }
            }
        }
    }
}
