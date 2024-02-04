#!/bin/bash

cargo fuzz build --release --fuzz-dir . --target-dir target-libfuzzer --sanitizer none --strip-dead-code fuzzer