# Aether Communication Protocol

## Purpose

Aether uses the repository as a durable communication environment between three roles:

1. **Creator / Architect (Virt)** — defines the evolution vector, higher-level goals, constraints, and acceptance criteria.
2. **Agent / Aether** — the repository-native process that executes bounded work and reports through persistent state, logs, commits, and clock ticks.
3. **User / External Environment** — observes, interacts with, and supplies external/runtime evidence to the environment.

The repository is the durable boundary. Chat is an interaction surface, not the system of record.

## Channels

| Channel | Direction | Meaning | Durable record |
|---|---|---|---|
| GitHub Issues / Discussions | Creator/User -> Aether | Requests, decisions, questions, acceptance criteria | issue/discussion reference + communication event |
| Git commits | Aether/Virt -> environment | Atomic implementation and state transitions | commit SHA + diff |
| GitHub Actions | Aether -> environment | Execution, verification, clock wakeups | workflow run/job/summary |
| QEMU serial log | Aether -> environment | Runtime observations from the reproducible lab | serial log + analyzed report |
| `.state/current_state.json` | Aether -> all roles | Current durable state, active intent, last communication event | state commit |
| `.state/history.json` | Aether -> all roles | Bounded event history and evidence | state commit |

## Event envelope

Communication events use these fields when persisted:

- `event_id`: monotonically increasing repository-local identifier.
- `type`: `request`, `decision`, `response`, `observation`, or `wake`.
- `actor`: `creator-architect`, `agent-aether`, or `user-external`.
- `channel`: one of the channels above.
- `observed_at`: UTC timestamp.
- `payload`: concise structured data; large evidence stays in its native log/artifact.
- `correlation_id`: optional identifier connecting a response/observation to a request.

The event envelope is metadata about communication, not a replacement for source code, CI logs, or hardware evidence.

## Role boundaries

### Creator / Architect

May define goals, priorities, constraints, and acceptance criteria. Does not get represented as autonomous runtime activity merely because a model inferred an intention.

### Agent / Aether

May execute only bounded, auditable repository actions. Its wakeups and observations are persisted. A clock tick is a wake event, not permission to invent a new goal.

### User / External Environment

May provide interactive input or real-world evidence. External observations must remain distinguishable from QEMU/CI evidence.

## Integration with the existing state loop

The protocol is intentionally attached to the existing mechanisms rather than creating a second state system:

- `.aether/tools/clock_tick.py` records a `wake` event whenever the repository clock advances. The event is persisted together with the existing clock state.
- `.aether/tools/analyze_run.py` records an `observation` event when a QEMU serial run is analyzed. The event is linked to the run ID and commit SHA.
- Both tools update `communication.last_event` in `.state/current_state.json`.
- `.state/history.json` remains the bounded event journal. Runtime evidence is referenced, not copied wholesale.
- The existing workflows remain the execution triggers: `aether_clock.yml` wakes the agent; `aether_evolution.yml` produces QEMU evidence.

## Request lifecycle

1. Creator/User expresses a request through a durable channel.
2. The request receives a communication event and, when work begins, a correlation ID.
3. Aether evaluates the request against current state and operating rules.
4. Aether performs one bounded change at a time.
5. Commit, Actions, QEMU, and analysis produce durable response/observation evidence.
6. The current state records the latest communication event and next action.
7. The history journal retains the bounded trail so the conversation does not depend on chat recall.

## Safety and truthfulness

- Documentation is not runtime evidence.
- A model statement is not a runtime event until persisted by an explicit repository action.
- QEMU, CI, and physical AH532 evidence remain separate.
- A wakeup never authorizes unbounded self-propagation.
- Missing external input or authorization remains an explicit blocked state.
- Communication history is bounded and append-oriented; old evidence is not silently rewritten.

## Current foundation

This protocol is a foundation, not a claim of full live chat ingestion. GitHub Issues/Discussions are the durable human ingress; an adapter that imports those events into `.state/` can be added as a later, separately verified change. The current clock and QEMU analyzer already provide two durable outbound event sources.
