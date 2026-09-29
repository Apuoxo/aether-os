# Aether Native Media Player

## Current status

**UI and media pipeline are implemented; end-to-end audible HDA playback is not yet verified on the AH532.**

The intended native path is:

`VFS/file -> MP3 decoder -> PCM ring -> HDA DMA -> codec/pin -> speakers`

The project currently targets **MP3** for the native media-player work. Other formats are not a current acceptance requirement.

## Current implementation

- native Media Player window;
- playlist/file entries;
- play/pause/stop state path;
- seek/time state;
- volume/mute/repeat/shuffle state;
- bounded MP3 decoding through the vendored `minimp3` implementation;
- PCM buffering structures;
- HDA controller discovery and MMIO;
- HDA reset/codec-verb infrastructure;
- HDA stream/BDL/PCM bring-up code.

## Current hardware blocker

The latest AH532 `AUD` diagnostic reported:

- `HDA_FOUND=YES`
- `MMIO=READY`
- `BAR0=F1610000`
- `STREAM_READY=NO`
- `RUNNING=NO`
- `BASE=0`
- `FMT=0`
- `CTL=0`
- `STAT=0`
- `LPIB=0`
- `BDL=0`
- `TOTAL=0`
- `NEXT=0`

Therefore controller detection is proven, but the playback stream is not yet running. Pressing Play cannot be treated as proof of audio output until the HDA stream state advances and the AH532 speakers produce PCM.

## Required next hardware sequence

1. establish codec discovery/NID topology;
2. allocate and verify BDL/DMA memory;
3. configure a valid PCM stream format;
4. program stream registers;
5. start RUN;
6. verify LPIB/position progress;
7. verify codec output/pin path;
8. hear and record actual PCM playback on AH532.

The existing diagnostic path must remain usable while this is implemented.

## Acceptance

For the current MP3 milestone, completion requires:

- green CI build;
- QEMU boot/regression evidence where applicable;
- AH532 HDA controller/codec evidence;
- actual MP3 PCM output on AH532;
- pause/resume/stop/restart;
- seek;
- volume/mute state reaching the output path;
- no host-disk writes.

A player window, progress animation, or controller-detection message is not evidence of audible playback.
