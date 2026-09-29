# Aether OS — Current Source Snapshot

This file identifies the source tree that documentation describes.

- Current `main` commit: `0bb996ed5ab809ae6c73709262ea964e7c3f3224`
- Snapshot basis: current GitHub `main`, not the historical `aether-os-sources.zip`.
- Current Explorer: `kernel/src/files_mgr.rs`
- Current desktop: `kernel/src/desktop.rs`
- Current NTFS: `kernel/src/fs_ntfs.rs`
- Current partition layer: `kernel/src/part.rs`
- Current shell: `kernel/src/shell.rs`
- Current GUI-terminal command routing: `kernel/src/desktop.rs` (`run_cmd()`)
- Current HDA/audio driver: `kernel/src/drivers/audio.rs`
- Current Media Player: `kernel/src/media_player.rs`
- Current MP3 decoder: `kernel/src/mp3_decoder/`
- Current Wi-Fi driver: `kernel/src/drivers/wifi.rs`
- Current Intel display paths: `kernel/src/drivers/intel_igpu.rs`, `intel_kms.rs`, `video.rs`

The historical source archive is not a substitute for the current tree. When a snapshot marker becomes stale, update this file in the same documentation-maintenance commit that establishes the new source baseline.
