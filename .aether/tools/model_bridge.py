#!/usr/bin/env python3
"""Bidirectional Aether AI model bridge.

Transport: QEMU COM1 <-> TCP socket <-> this process.
Model: GitHub Copilot CLI (or a compatible command exposed as AETHER_MODEL_COMMAND).
The bridge never modifies the guest; it only translates framed text messages.
"""

import argparse
import os
import shlex
import socket
import subprocess
import time


def log(line: str) -> None:
    print(line, flush=True)


def model_answer(message: str, model: str) -> str:
    command = os.environ.get("AETHER_MODEL_COMMAND")
    if command:
        argv = shlex.split(command)
        if not argv:
            raise RuntimeError("AETHER_MODEL_COMMAND is empty")
        argv += [message]
    else:
        argv = [
            "copilot", "-p",
            (
                "You are the external language model connected to Aether OS. "
                "Answer the user's message directly and concisely. "
                "Do not claim to be running inside the kernel. "
                "Return one plain-text line, maximum 90 characters. "
                "User message: " + message
            ),
            "-s", "--no-ask-user", "--model", model,
        ]
    result = subprocess.run(
        argv, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        timeout=45, check=False, env=dict(os.environ),
    )
    answer = " ".join(result.stdout.strip().split())
    if result.returncode != 0 or not answer:
        detail = answer[:70] if answer else "no model output"
        raise RuntimeError(f"external model request failed rc={result.returncode} detail={detail}")
    return answer[:90]


def connect(host: str, port: int, timeout: float = 30.0) -> socket.socket:
    deadline = time.monotonic() + timeout
    while True:
        try:
            sock = socket.create_connection((host, port), timeout=2.0)
            sock.settimeout(1.0)
            return sock
        except OSError:
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.25)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=45454)
    ap.add_argument("--inject")
    ap.add_argument("--model", default="auto")
    args = ap.parse_args()

    # Wait for the guest's explicit READY marker before injecting input.
    # The TCP socket can be connected long before the kernel reaches its
    # desktop/AI loop; sending AI_IN immediately can otherwise be lost before
    # the guest UART is initialized.
    sock = connect(args.host, args.port)
    buf = b""
    log("AI_STATUS:CONNECTED")
    sock.sendall(b"AI_STATUS:ACTIVE\n")
    log("AI_STATUS:ACTIVE")
    payload = args.inject.replace("\r", " ").replace("\n", " ")[:90] if args.inject else None
    deadline = None
    try:
        while True:
            try:
                chunk = sock.recv(1024)
            except socket.timeout:
                if deadline is not None and time.monotonic() >= deadline:
                    raise TimeoutError("model bridge response timeout")
                continue
            if not chunk:
                return 0
            buf += chunk
            while b"\n" in buf:
                raw, buf = buf.split(b"\n", 1)
                line = raw.decode("utf-8", "replace").rstrip("\r")
                if line:
                    log(line)
                if line == "AI_STATUS:READY" and payload is not None:
                    sock.sendall(("AI_IN:" + payload + "\n").encode("utf-8", "replace"))
                    log("AI_IN:" + payload)
                    deadline = time.monotonic() + 60.0
                if line.startswith("AI_ACK:"):
                    log("AI_BRIDGE: response acknowledged")
                    # The guest may emit an internal RUNTIME acknowledgement
                    # before answering the externally injected U request.
                    # Only the U-channel acknowledgement completes this run.
                    if args.inject and line.startswith("AI_ACK:REQ=U"):
                        return 0
                if line.startswith("AI_REQ:"):
                    request = line[7:].strip()
                    parts = request.split(" ", 1)
                    request_tag = parts[0] if parts and parts[0].startswith("REQ=") else ""
                    message = parts[1] if len(parts) == 2 else request
                    try:
                        answer = model_answer(message, args.model)
                        wire = "AI_RES:" + request_tag + ":" + answer.replace("\r", " ").replace("\n", " ") + "\n"
                    except Exception as exc:
                        wire = "AI_RES:" + request_tag + ":MODEL_ERROR " + str(exc).replace("\r", " ").replace("\n", " ")[:70] + "\n"
                        log(wire.rstrip("\n"))
                    # Pace bytes so a polled 16550-compatible UART cannot
                    # overrun its small RX FIFO before the guest drains it.
                    time.sleep(0.05)
                    encoded = wire.encode("utf-8", "replace")
                    for byte in encoded:
                        sock.sendall(bytes((byte,)))
                        time.sleep(0.002)
                    log(wire.rstrip("\n"))
                    if args.inject:
                        deadline = time.monotonic() + 15.0
    finally:
        sock.close()


if __name__ == "__main__":
    raise SystemExit(main())
