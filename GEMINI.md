# Aether OS agent instructions

Follow the same repository execution policy as the primary agent instructions.

When a GitHub/Aether task is started or continued:
- inspect current HEAD and relevant files first;
- perform every safe, directly necessary, clearly implied action that is available;
- use one controlled logical change per commit;
- verify SHA, parent, main, diff, CI, logs, and relevant artifacts after changes;
- diagnose failures from actual evidence before changing code;
- do not guess or invent repository, build, hardware, or test state;
- stop only when complete, when user input is genuinely required, or when the next action would be risky/destructive/out of scope.

Correctness before speed; preserve boot and desktop behavior unless explicitly changing them.
