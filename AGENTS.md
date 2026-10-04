# Aether OS agent instructions

## Mandatory execution rule

When the user starts or continues a GitHub/Aether task, immediately perform every safe, directly necessary, and clearly implied action that can be completed with the available GitHub tools. Do not stop merely because the user did not explicitly name each intermediate step.

The working rule is:

`inspect -> decide -> change -> commit -> verify -> CI -> inspect evidence -> continue`

Continue automatically through this chain until:
- the task is actually complete;
- a real user decision or missing external input is required; or
- the next action would be risky, destructive, speculative, or outside the requested scope.

Never invent repository state, CI results, files, APIs, or test evidence.

## GitHub operating rules

1. Treat `main` and the current repository state as authoritative. Before changing anything, verify the current HEAD and inspect the relevant source/configuration.
2. Make one controlled logical change per commit. Do not create speculative chains of commits.
3. Before editing an existing file, read the current version. Keep unrelated files and behavior unchanged.
4. After every commit, verify:
   - exact commit SHA;
   - parent SHA;
   - `main` points to the new commit;
   - changed files/diff;
   - relevant GitHub Actions runs.
5. When Actions fails, automatically inspect the failed workflow/job/step and logs, determine the concrete cause, and make the smallest justified correction. Do not guess at fixes.
6. When diagnostics or artifacts are produced, retrieve and inspect the evidence when it is relevant to the next decision.
7. Prefer repository evidence over memory. Prefer current source over old documentation. Prefer official documentation when a GitHub behavior or tool capability is uncertain.
8. Do not claim hardware or runtime behavior from CI alone. Keep build evidence, QEMU evidence, and real-AH532 evidence distinct.
9. If a required GitHub operation is unavailable through one interface, inspect the available GitHub capabilities and use a safe supported alternative rather than simply stopping.
10. Do not force-push, delete, overwrite unrelated work, or perform other destructive operations without explicit authorization.

## Aether-specific quality gate

Correctness before speed. Before each code change, establish the current architecture and dependencies. After each change, validate the build/test path that can actually validate it. Preserve boot and desktop behavior unless the requested change explicitly targets them.

For video/graphics work, respect the current canonical framebuffer architecture and the project's staged one-change-at-a-time workflow.

For all work, distinguish:
- code = current behavior;
- tests/CI/hardware logs = evidence;
- documentation/roadmap = explanation or intended future work.

If the obvious next safe GitHub action is already possible, perform it now rather than asking the user to repeat the command.
