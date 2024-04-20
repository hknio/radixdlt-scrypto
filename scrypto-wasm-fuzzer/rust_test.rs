// Rust equivalent of including sanitizer headers and using the uint8_t type
use std::io::{self, Write}; // to use println! and flush stdout

// Declare global mutable pointers using static mut and unsafe code.
static mut START: *const u8 = std::ptr::null();
static mut STOP: *const u8 = std::ptr::null();

// Rust version of the C extern function
#[no_mangle]
pub extern "C" fn __sanitizer_cov_8bit_counters_init(start: *const u8, stop: *const u8) {
    println!("called");
    unsafe {
        START = start;
        STOP = stop;
    }
}

// Rust function equivalent to the empty foo function in C++
fn foo() {}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        foo();
    }

    // Using unsafe block to access and dereference the raw pointers
    unsafe {
        println!("Results");
        let mut x = START;
        while x < STOP {
            // Dereferencing a raw pointer in Rust requires an unsafe block
            println!("hit: {}", *x);
            x = x.offset(1); // Move to the next byte
        }
    }

    Ok(())
}