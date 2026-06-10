#!/usr/bin/env python3

import concurrent.futures
import time


def string_upper_work(value: str) -> str:
    return value.upper()


def run_serial(values):
    start = time.perf_counter()
    checksum = 0
    iterations = 0
    while iterations < 500:
        mapped = [string_upper_work(v) for v in values]
        checksum += sum(len(item) for item in mapped)
        iterations += 1
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    return elapsed_ms, checksum


def run_process_pool(values):
    start = time.perf_counter()
    checksum = 0
    with concurrent.futures.ProcessPoolExecutor() as executor:
        iterations = 0
        while iterations < 500:
            mapped = list(executor.map(string_upper_work, values, chunksize=32))
            checksum += sum(len(item) for item in mapped)
            iterations += 1
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    return elapsed_ms, checksum


def main():
    values = [
        "value_1", "value_2", "value_3", "value_4", "value_5", "value_6", "value_7", "value_8",
        "value_9", "value_10", "value_11", "value_12", "value_13", "value_14", "value_15", "value_16",
    ]

    serial_ms, serial_checksum = run_serial(values)
    process_pool_ms, process_pool_checksum = run_process_pool(values)

    if serial_checksum != process_pool_checksum:
        raise RuntimeError(
            f"Checksum mismatch serial={serial_checksum} process_pool={process_pool_checksum}"
        )

    print(f"PYTHON_SERIAL_MS={serial_ms:.6f}")
    print(f"PYTHON_PROCESS_POOL_MS={process_pool_ms:.6f}")
    print(f"PYTHON_PROCESS_POOL_CHECKSUM={process_pool_checksum}")


if __name__ == "__main__":
    main()