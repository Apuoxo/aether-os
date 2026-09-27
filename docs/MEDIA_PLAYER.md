# Aether Native Media Player

Status: implementation started.

The media player is a native Aether application, not a cosmetic desktop window. The final path is:

`VFS/file -> bounded demux/decoder -> PCM ring -> mixer -> HDA DMA -> codec/pin -> speakers`

## Current foundation

Commit `d6e48dc6c915ad410df72235a0cb2132f41f3607` extends the existing audio probe without touching HDA MMIO. It records the HDA controller BDF and BAR0 candidate and keeps the PC speaker strictly diagnostic.

This is deliberate: the next hardware stage needs an owned MMIO mapping and bounded controller initialization before CORB/RIRB or stream DMA can be touched.

## Player architecture

### 1. Media library boundary

The decoder layer must be isolated from the kernel's hardware layer. Candidate reference: Symphonia 0.6.x, a pure-Rust demux/decoder framework supporting MP3, FLAC, OGG/Vorbis, WAV and additional formats. It is MPL-2.0.

Do not copy decoder implementation from third-party projects. Integrate upstream crates where their no_std/alloc requirements can be satisfied; otherwise implement format-specific support from the relevant specifications or use an explicitly compatible implementation.

### 2. Streaming

No whole-file decode. The player uses bounded buffers:
- input read window;
- compressed packet queue;
- decoded PCM ring;
- HDA DMA period buffers.

The decoder must be able to pause, resume, seek, and recover from malformed input without panicking.

### 3. HDA backend

The hardware sequence is:
1. PCI discovery and BAR ownership.
2. MMIO mapping.
3. Controller reset and capability readout.
4. CORB/RIRB command transport.
5. Codec discovery.
6. Widget/pin/path discovery.
7. PCM format negotiation.
8. BDL allocation.
9. Output stream configuration.
10. DMA start/stop/pause and position tracking.
11. Interrupt-driven refill.
12. Volume/mute through codec verbs where supported.

Intel's HDA specification defines the CORB/RIRB command path and stream descriptors/BDLs used for DMA.

### 4. Native GUI

The player window will use Aether's existing compositor/window system rather than a second GUI framework.

Required controls:
- album art / track information;
- playlist;
- play/pause/stop;
- previous/next;
- seek bar;
- elapsed/total time;
- volume/mute;
- repeat/shuffle;
- file open;
- decoder/output error state.

The UI is functional only when its actions reach the player state machine and ultimately the audio backend.

## Completion gates

The feature is not considered complete until all of these are demonstrated:
- clean kernel/ISO build;
- QEMU boot regression;
- AH532 HDA controller evidence;
- actual PCM output on AH532;
- WAV playback;
- MP3 playback;
- at least one lossless format;
- seek;
- pause/resume;
- stop/restart;
- volume/mute;
- playlist transitions;
- malformed/unsupported-file handling;
- no host-disk writes;
- bounded memory/resource report.

A window, command, fake progress bar, or PC-speaker beep is not accepted as evidence of a working media player.