"""Shared helpers for repository-native communication events."""


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
