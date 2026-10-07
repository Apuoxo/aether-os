# Aether OS architecture

## 1. Identity

Aether is an AI-first system. The operating environment is the AI's execution and interaction substrate; the desktop and individual applications are not the architectural center.

## 2. Layer model

~~~text
AETHER AI
    |
    | privileged IPC / system API
    v
AETHER CORE
    |
    | process / memory / device / storage / network / graphics / system state
    v
LINUX FOUNDATION
    |
    v
HARDWARE
~~~

Linux is the initial foundation because it already provides mature CPU, memory, driver and device support. Aether-specific layers must not unnecessarily duplicate those mechanisms.

## 3. Linux compatibility is a contract

Aether's future native kernel is not allowed to achieve its AI-first architecture by abandoning the Linux ecosystem. Compatibility is a core requirement:

- Linux ELF applications must remain runnable.
- Linux/POSIX system interfaces required by normal applications must remain available.
- Linux graphics, audio, networking and IPC interfaces needed by desktop/media/browser software must remain usable.
- Existing Linux drivers should remain usable while native Aether drivers are introduced where useful.

This means Aether may replace kernel internals without breaking the interface expected by Linux software.

## 4. Self-development

Aether AI is intended to have authority to inspect and modify Aether Core and, ultimately, the kernel. The architecture therefore needs a controlled engineering path for:

1. inspect current system and source;
2. generate or modify code/configuration;
3. build a candidate system;
4. boot/test the candidate;
5. validate compatibility and system health;
6. deploy or roll back the candidate.

The recovery/rollback machinery exists so self-modification can continue after a failed candidate; it is not a substitute for AI authority.

## 5. Current implementation boundary

The current boot image contains Linux plus a minimal Aether Core bootstrap. The Core is a real privileged process with a Unix-domain IPC endpoint. It currently exposes only minimal health/system-state operations. AI integration, persistent system state, broad system APIs, desktop integration and Linux application compatibility work are subsequent milestones.
