#!/usr/bin/env python3
import json
import subprocess
import sys


def write_message(stdin, payload):
    encoded = json.dumps(payload).encode("utf-8")
    header = f"Content-Length: {len(encoded)}\r\n\r\n".encode("utf-8")
    stdin.write(header)
    stdin.write(encoded)
    stdin.flush()


def read_message(stdout):
    content_length = None
    while True:
        line = stdout.readline()
        if not line:
            return None
        if line in (b"\r\n", b"\n"):
            break
        decoded = line.decode("utf-8").strip()
        if decoded.lower().startswith("content-length:"):
            content_length = int(decoded.split(":", 1)[1].strip())

    if content_length is None:
        raise RuntimeError("missing Content-Length header")

    body = stdout.read(content_length)
    return json.loads(body.decode("utf-8"))


def main():
    if len(sys.argv) != 2:
        print("usage: python_client.py <kujo_binary>", file=sys.stderr)
        return 2

    binary = sys.argv[1]
    proc = subprocess.Popen(
        [binary, "lsp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    try:
        write_message(proc.stdin, {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {},
        })
        response = read_message(proc.stdout)
        if response is None or "result" not in response:
            raise RuntimeError("initialize did not return result")

        write_message(proc.stdin, {
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {},
        })

        write_message(proc.stdin, {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
            "params": None,
        })
        shutdown_response = read_message(proc.stdout)
        if shutdown_response is None or "result" not in shutdown_response:
            raise RuntimeError("shutdown did not return result")

        write_message(proc.stdin, {
            "jsonrpc": "2.0",
            "method": "exit",
        })

        proc.wait(timeout=5)
        if proc.returncode != 0:
            raise RuntimeError(f"kujo lsp exited with code {proc.returncode}")

        return 0
    finally:
        if proc.poll() is None:
            proc.kill()


if __name__ == "__main__":
    sys.exit(main())
