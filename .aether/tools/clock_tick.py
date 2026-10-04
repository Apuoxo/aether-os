#!/usr/bin/env python3
"""Advance the repository clock by one bounded wake-up tick."""
import argparse
import json
import os
from datetime import datetime, timezone
from pathlib import Path

from communication import record_communication_event

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--state", default=".state/current_state.json")
    ap.add_argument("--history", default=".state/history.json")
    args = ap.parse_args()

    state_path = Path(args.state)
    history_path = Path(args.history)
    state = json.loads(state_path.read_text())
    history = json.loads(history_path.read_text()) if history_path.exists() else {"schema": 1, "entries": []}

    now = datetime.now(timezone.utc)
    now_iso = now.isoformat().replace("+00:00", "Z")
    clock = state.setdefault("clock", {})
    previous_iso = clock.get("last_wakeup")

    delta = 0.0
    if previous_iso:
        previous = datetime.fromisoformat(previous_iso.replace("Z", "+00:00"))
        delta = max(0.0, (now - previous).total_seconds())

    tick = int(clock.get("tick_counter", 0)) + 1
    total = float(clock.get("total_elapsed_seconds", 0.0)) + delta

    clock.update({
        "tick_counter": tick,
        "last_wakeup": now_iso,
        "delta_seconds": round(delta, 3),
        "total_elapsed_seconds": round(total, 3),
        "clock_source": "github-actions-runner-system-time",
        "last_run_id": os.environ.get("GITHUB_RUN_ID", "unknown"),
    })
    state["updated"] = now.strftime("%Y-%m-%d")
    state["last_wake_reason"] = os.environ.get("GITHUB_EVENT_NAME", "unknown")

    communication_event = record_communication_event(
        state,
        "wake",
        "agent-aether",
        "github_actions",
        {
            "observed_at": now_iso,
            "tick_counter": tick,
            "delta_seconds": round(delta, 3),
            "run_id": os.environ.get("GITHUB_RUN_ID", "unknown")
        },
        correlation_id=f"clock-{os.environ.get('GITHUB_RUN_ID', 'unknown')}"
    )

    entry = {
        "type": "clock_tick",
        "event_id": communication_event["event_id"],
        "communication_event": communication_event,
        "tick_counter": tick,
        "observed_at": now_iso,
        "delta_seconds": round(delta, 3),
        "total_elapsed_seconds": round(total, 3),
        "run_id": os.environ.get("GITHUB_RUN_ID", "unknown"),
        "commit": os.environ.get("GITHUB_SHA", "unknown"),
        "trigger": os.environ.get("GITHUB_EVENT_NAME", "unknown"),
    }
    history.setdefault("entries", []).append(entry)
    history["entries"] = history["entries"][-100:]
    history["schema"] = 1
    history["last_updated"] = now_iso
    history["last_tick_counter"] = tick
    history["last_communication_event"] = communication_event["event_id"]

    state_path.write_text(json.dumps(state, indent=2, ensure_ascii=False) + "\n")
    history_path.write_text(json.dumps(history, indent=2, ensure_ascii=False) + "\n")

    print(json.dumps(entry, indent=2, ensure_ascii=False))

if __name__ == "__main__":
    main()
