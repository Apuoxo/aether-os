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
        raise RuntimeError("external model request failed")
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
    ap.add_argument("--model", default="gpt-5.4")
    args = ap.parse_args()

    sock = connect(args.host, args.port)
    buf = b""
    ready_deadline = time.monotonic() + 45.0
    guest_ready = False
    while not guest_ready:
        try:
            chunk = sock.recv(1024)
        except socket.timeout:
            if time.monotonic() >= ready_deadline:
                raise TimeoutError("guest AI transport ready timeout")
            continue
        if not chunk:
            raise RuntimeError("guest closed AI transport before ready")
        buf += chunk
        while b"\n" in buf:
            raw, buf = buf.split(b"\n", 1)
            line = raw.decode("utf-8", "replace").rstrip("\r")
            if line:
                log(line)
            if line == "AI_STATUS:READY":
                guest_ready = True
                break

    sock.sendall(b"AI_STATUS:ACTIVE\n")
    log("AI_STATUS:ACTIVE")
    if args.inject:
        payload = args.inject.replace("\r", " ").replace("\n", " ")[:90]
        sock.sendall(("AI_IN:" + payload + "\n").encode("utf-8", "replace"))
        log("AI_IN:" + payload)

    deadline = time.monotonic() + 60.0 if args.inject else None
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
                if line.startswith("AI_REQ:"):
                    try:
                        answer = model_answer(line[7:].strip(), args.model)
                        wire = "AI_RES:" + answer.replace("\r", " ").replace("\n", " ") + "\n"
                    except Exception as exc:
                        wire = "AI_RES:MODEL_ERROR " + str(exc).replace("\r", " ").replace("\n", " ")[:70] + "\n"
                        log(wire.rstrip("\n"))
                    sock.sendall(wire.encode("utf-8", "replace"))
                    log(wire.rstrip("\n"))
                    if args.inject:
                        log("AI_BRIDGE: request served")
                        return 0
    finally:
        sock.close()


if __name__ == "__main__":
    raise SystemExit(main())
