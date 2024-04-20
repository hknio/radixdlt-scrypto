use scrypto::prelude::*;
use scrypto::types::Slice;

static mut START: *const u8 = std::ptr::null();
static mut STOP: *const u8 = std::ptr::null();

#[no_mangle]
pub extern "C" fn __sanitizer_cov_8bit_counters_init(start: *const u8, stop: *const u8) {
    unsafe {
        START = start;
        STOP = stop;
    }
}

#[no_mangle]
pub unsafe extern "C" fn dump_coverage_counters() -> Slice {
    let length = STOP.offset_from(START) as usize;
    Slice::new(START as u32, length as u32)
}

#[blueprint]
mod fuzz_blueprint {
    struct FuzzBlueprint;

    impl FuzzBlueprint {
        pub fn fuzz(input: Vec<u8>) -> Vec<u8> {
            if input.len() == 2 {
                info!("OK");
                if input[0] == 'X' as u8 {
                    info!("OK2");
                    if input[1] == 'Y' as u8 {
                        info!("OK3");
                    }
                }
            }

            let map = scrypto_decode::<IndexMap<u32, IndexSet<(String, Decimal)>>>(&input);
            if map.is_ok() {
                let map = map.unwrap();;
                if map.len() > 0 {
                    if map.contains_key(&55) {
                        panic!("wow");
                    }
                }
            }
            unsafe { 
                let length = STOP.offset_from(START) as usize;
                let slice = std::slice::from_raw_parts(START, length);
                slice.to_vec()         
            }
        }

        pub fn get_counters_size() -> usize {
            unsafe {
                STOP.offset_from(START) as usize
            }
        }
    }
}

