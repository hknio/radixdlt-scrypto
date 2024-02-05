import os
import subprocess
import re

def run_command(command, cwd=None):
    """Run a shell command in a specific directory."""
    print(f"Running command: {command}")
    return subprocess.check_output(command, shell=True, cwd=cwd, text=True)

def main():
    original_dir = os.getcwd()

    # Step 1
    os.chdir("radix-runtime-fuzzer-tools")
    run_command("cargo build --release --bin validator")
    run_command("cargo build --release --bin extractor")
    os.chdir(original_dir)

    # Step 2
    raw_txs_path = os.path.join(original_dir, "radix-runtime-fuzzer-test-cases/raw")
    if not os.path.exists(raw_txs_path):
        os.makedirs(raw_txs_path)

    # Step 3
    os.chdir("radix-engine-tests")
    test_list_output = run_command("cargo test --features radix_runtime_logger -- --list")
    test_names = [line.split(': test')[0] for line in test_list_output.splitlines() if line.endswith(": test")]

    # Step 4
    for test_name in test_names:
        sanitized_test_name = re.sub('[^a-z]', '_', test_name.lower())
        txs_bin_name = f"{raw_txs_path}/{sanitized_test_name}.bin"
        os.environ["RADIX_RUNTIME_LOGGER_FILE_NAME"] = txs_bin_name
        try:
            run_command(f"cargo test --features radix_runtime_logger -- {test_name}")
        except subprocess.CalledProcessError:
            if os.path.exists(txs_bin_name):
                os.remove(txs_bin_name)
            continue

        if not os.path.exists(txs_bin_name):
            continue

        try:
            run_command(f"{original_dir}/target/release/validator '{txs_bin_name}'")
        except subprocess.CalledProcessError:
            os.remove(txs_bin_name)
            continue
            
    os.chdir(original_dir)

    # Step 5
    extracted_txs_path = os.path.join(original_dir, "radix-runtime-fuzzer-test-cases/extracted")
    if not os.path.exists(extracted_txs_path):
        os.makedirs(extracted_txs_path)
    run_command(f"{original_dir}/target/release/validator '{raw_txs_path}' '{extracted_txs_path}'")
    
    print(f"Now you can build fuzzer and use {extracted_txs_path} as corpus.")

if __name__ == "__main__":
    main()