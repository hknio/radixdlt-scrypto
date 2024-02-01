use std::fs::File;

use radix_runtime_fuzzer::*;
use radix_runtime_fuzzer_derive::*;
use radix_engine_common::prelude::{scrypto_encode, scrypto_decode};

struct Test;

trait TestTrait {
    fn test(&self, a: u32, b: u32) -> Result<u32, ()>;

    fn test2(&self, a: u32) -> Result<u32, ()>;

    fn test3(&self) -> Result<(), ()>;
}

#[runtime_fuzzer]
impl TestTrait for Test {
    fn test(&self, a: u32, mut b: u32) -> Result<u32, ()> {
        b += 1;
        Ok(a + b)
    }

    fn test2(&self, a: u32) -> Result<u32, ()> {
        if a < 1 {
            Err(())
        } else {
            Ok(a + 2)
        }
    }

    fn test3(&self) -> Result<(), ()> {
        Ok(())
    }
}

#[test]
fn bassic_tests() {
    let mut test = Test;
    assert_eq!(test.test(1, 2), Ok(4));
    assert_eq!(test.test2(0), Err(()));
    assert_eq!(test.test2(1), Ok(3));
    //assert_eq!(test.test3(), Ok(()));

    // read file log.txt
    let mut file: File = OpenOptions::new()
        .read(true)
        .open("log.txt")
        .expect("failed to open log.txt");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("failed to read log.txt");
    let lines: Vec<&str> = contents.split("\n").collect();
    for line in lines {
        test.fuzz(line.to_string());
    }
}
