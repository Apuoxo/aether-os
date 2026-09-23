# Aether OS — Build Trigger Log

## Known working build trigger

The Aether OS ISO workflow is:

- Workflow: **Aether OS build**
- Workflow file: `.github/workflows/build.yml`
- Workflow ID: `365334131`
- Trigger: `push` to `main` (also pull requests)
- ISO job builds `kernel/build/aether.iso`
- Artifact name: `aether-iso`

### How to start a build from the GitHub connector

If there is no direct `workflow_dispatch` action available, use the existing **push trigger**:

1. Inspect the current `main` HEAD and its tree.
2. If a rebuild is needed without source changes, create a **new commit pointing to the exact same tree SHA** as the desired source commit.
3. Move/update `main` to that new commit with a non-force update.
4. GitHub Actions sees the push to `main` and starts **Aether OS build** automatically.
5. Verify that the new run's `head_sha` is the new commit, and that the resulting artifact comes from that run.
6. Never confuse the previous successful ISO run with the new build.

### Example used on 2026-09-24

Desired source commit:
`cef44728cc05d0bbbf19f52e12cb7a971aaf4868`

Its tree:
`e63ab8e77f7771fe674bfb4b88dc335d008f116d`

Technical rebuild commit:
`d35216935e2525650a7735ef072ce463aea0f88e`

The technical commit uses the **same tree** as the desired source commit, so it exists only to produce a new `push` event and rebuild the ISO.

### Important

Do not tell the user that the assistant cannot start a build merely because `workflow_dispatch` is unavailable. The repository's existing `push → main` trigger is sufficient when repository write access is available.

Do not create arbitrary source changes just to trigger CI. Reuse the exact desired tree when a source-identical rebuild is required.
