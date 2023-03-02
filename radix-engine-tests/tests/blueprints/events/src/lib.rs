use scrypto::prelude::*;

#[derive(ScryptoSbor)]
struct CustomEvent {
    number: u64,
}

#[blueprint]
mod event_store_visibility {
    struct EventStoreVisibility;

    impl EventStoreVisibility {
        pub fn emit_event(number: u64) {
            Runtime::emit_event(CustomEvent { number });
        }
    }
}
