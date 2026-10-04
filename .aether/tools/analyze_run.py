#!/usr/bin/env python3
"""Analyze one Aether QEMU serial log and persist bounded experience."""
import argparse, json, os, re
from datetime import datetime, timezone
from pathlib import Path

def record_communication_event(state, event_type, actor, channel, payload, correlation_id=None):
    communication = state.setdefault("communication", {
        "schema": 1,
        "roles": {},
        "channels": {},
        "event_contract": {},
        "sequence": 0
    })
    sequence = int(communication.get("sequence", 0)) + 1
    event = {
        "event_id": f"comm-{sequence:06d}",
        "type": event_type,
        "actor": actor,
        "channel": channel,
        "observed_at": payload["observed_at"],
        "payload": payload
    }
    if correlation_id:
        event["correlation_id"] = correlation_id
    communication["sequence"] = sequence
    communication["last_event"] = event
    return event

def marker(text, *needles):
    return any(n.lower() in text.lower() for n in needles)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--log", required=True)
    ap.add_argument("--state", required=True)
    ap.add_argument("--history", required=True)
    ap.add_argument("--report", required=True)
    ap.add_argument("--qemu-exit", type=int, required=True)
    args = ap.parse_args()

    log = Path(args.log).read_text(errors="replace") if Path(args.log).exists() else ""
    state_path, history_path, report_path = map(Path, (args.state, args.history, args.report))
    state = json.loads(state_path.read_text())
    history = json.loads(history_path.read_text()) if history_path.exists() else {"schema": 1, "entries": []}
    entries = history.setdefault("entries", [])

    run_id = os.environ.get("GITHUB_RUN_ID", "unknown")
    sha = os.environ.get("GITHUB_SHA", "unknown")
    observed_at = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    qemu_exit_ok = args.qemu_exit in (0, 124)
    panic = marker(log, "PANIC")
    desktop = marker(log, "Aether Desktop v1.1 XP", "Aether Desktop")
    banner = marker(log, "Aether OS Build")
    desktop_start = marker(log, "[DESKTOP] starting")
    ring3 = (
        marker(log, "entering Ring3", "Ring3 started", "Ring3 execution", "userspace Ring3")
        and not marker(log, "boot test deferred", "no Ring3")
    )
    outcome = "success" if qemu_exit_ok and not panic and banner and desktop else "failure"

    observations = []
    if desktop:
        observations.append("Aether Desktop initialization observed in QEMU serial output.")
    if desktop_start:
        observations.append("Native desktop start marker observed.")
    if not ring3:
        observations.append("No Ring3 execution evidence observed; QEMU validation remains pre-Ring3.")
    if marker(log, "IGPU] no Intel display controller"):
        observations.append("QEMU has no Intel Gen6 display controller; this is virtual-hardware evidence, not AH532 evidence.")
    if marker(log, "HDA controller not found"):
        observations.append("QEMU has no native HDA controller; audio hardware behavior is not validated here.")
    if marker(log, "no Intel WLAN"):
        observations.append("QEMU has no target Intel WLAN device; Wi-Fi hardware behavior is not validated here.")
    if marker(log, "USB] xHCI FAIL"):
        observations.append("QEMU xHCI validation is unavailable in this run.")
    if marker(log, "[MEDIA] embedded TEST.WAV opened directly"):
        observations.append("Embedded media file open path was reached.")
    if marker(log, "[FB] test pattern rendered"):
        observations.append("Framebuffer test pattern reached the render stage.")

    conclusion = (
        "QEMU boot reached Aether Desktop without PANIC; evidence is valid for kernel/ISO/QEMU "
        "startup but does not establish AH532 hardware behavior or Ring3 execution."
        if outcome == "success" else
        "QEMU validation did not satisfy all required startup markers; retain the failure as bounded experience."
    )
    next_step = (
        "Use the recorded QEMU evidence as the baseline and choose the next single controlled validation."
        if outcome == "success" else
        "Inspect the failed marker or exit condition before making another code change."
    )

    communication_event = record_communication_event(
        state,
        "observation",
        "agent-aether",
        "qemu_serial",
        {
            "observed_at": observed_at,
            "run_id": run_id,
            "commit": sha,
            "outcome": outcome,
            "qemu_exit": args.qemu_exit
        },
        correlation_id=f"qemu-{run_id}"
    )

    entry = {
        "type": "qemu_observation",
        "event_id": communication_event["event_id"],
        "communication_event": communication_event,
        "run_id": run_id,
        "commit": sha,
        "observed_at": observed_at,
        "outcome": outcome,
        "qemu_exit": args.qemu_exit,
        "checks": {
            "qemu_exit_accepted": qemu_exit_ok,
            "panic_absent": not panic,
            "build_banner": banner,
            "desktop_marker": desktop,
            "desktop_start": desktop_start,
            "ring3_evidence": ring3
        },
        "observations": observations,
        "conclusion": conclusion,
        "next_step": next_step
    }

    if any(e.get("run_id") == run_id for e in entries):
        raise SystemExit("run already recorded; refusing duplicate state mutation")

    entries.append(entry)
    history["schema"] = 1
    history["last_updated"] = observed_at
    history["last_run_id"] = run_id
    history["entries"] = entries[-50:]

    state["updated"] = observed_at[:10]
    state.setdefault("verification", {})
    state["verification"].update({
        "latest_evolution_run": run_id,
        "latest_evolution_commit": sha,
        "latest_evolution_outcome": outcome,
        "latest_evolution_qemu_exit": args.qemu_exit,
        "latest_evolution_evidence": "serial-log-analyzed"
    })
    state["current_hypotheses"][0]["status"] = "supported" if outcome == "success" else "provisional"
    state["next_step"] = next_step
    state["communication"]["last_event"] = communication_event
    state["last_experience"] = {
        "outcome": outcome,
        "conclusion": conclusion,
        "observations": observations
    }

    report = {
        "schema": 1,
        "run_id": run_id,
        "commit": sha,
        "observed_at": observed_at,
        "outcome": outcome,
        "qemu_exit": args.qemu_exit,
        "checks": entry["checks"],
        "observations": observations,
        "conclusion": conclusion,
        "next_step": next_step
    }

    for path, data in ((report_path, report), (history_path, history), (state_path, state)):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")

    print(json.dumps(report, indent=2, ensure_ascii=False))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
