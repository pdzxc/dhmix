# StreamMix

A free, open Voicemeeter-Potato-style mixer for streaming on Windows. No licence, no nag screen.

- **7 input strips**: HW 1–3 (microphones, interfaces), VIRT 1–3 (virtual cables that apps play into), PLAYER (soundboard + music)
- **7 output buses**: A1–A5 hardware outs (headphones, speakers, a second PC), B1–B2 virtual outs (what OBS, Discord or a call hears)
- **Per strip**: gain fader, pan, MONO, SOLO, MUTE, routing grid, and an FX chain of denoiser (RNNoise), noise gate, 4-band EQ (low cut, bass, mid, treble), compressor, echo and reverb
- **Per bus**: gain, mute, bass/treble tone, brick-wall limiter
- **Streaming extras**: soundboard with 9 pads and global hotkeys (Ctrl+Alt+1–9), music player with loop and seek, global mute hotkey (Ctrl+Alt+M), WAV recording of any bus, presets (JSON), state restored on next launch
- Engine runs at 48 kHz in 10 ms blocks; devices at other rates are resampled.

## Build on Windows

1. Install the Rust toolchain from <https://rustup.rs> (pick the default MSVC toolchain). When it asks, let it install the Visual Studio Build Tools with the "Desktop development with C++" workload.
2. In a terminal inside this folder:

```bat
cargo build --release
```

3. The app is at `target\release\streammix.exe`. Copy it anywhere; it has no other files.

The first build takes a few minutes (it compiles the GUI and audio libraries). Later builds take seconds.

## Virtual cables on Windows

Windows only lets a kernel driver create an audio device, and installing a driver needs Microsoft attestation signing with a paid EV certificate. StreamMix therefore does what Voicemeeter's competitors do: it uses any virtual cable that is already installed and treats its endpoints like devices.

Install one of these free cables:

- **VB-CABLE** (donationware, free to use): <https://vb-audio.com/Cable/>. It adds `CABLE Input` (a playback device) and `CABLE Output` (a recording device). The optional A+B pack adds two more cables.
- **Virtual Audio Driver** (open source, MIT): <https://github.com/VirtualDrivers/Virtual-Audio-Driver>. Same idea, signed releases on the Releases page.

Then wire it up:

| You want | Do this |
| --- | --- |
| Discord / game audio in your mix | In Windows Sound settings, set that app's output to `CABLE Input`. In StreamMix set VIRT 1's device to `CABLE Output`. |
| OBS / Discord to hear your mix | Set bus B1's device to `CABLE Input` of a *second* cable (for example `CABLE-A Input`). In OBS or Discord, choose that cable's `Output` as the microphone. |
| Hear everything yourself | Set bus A1's device to your headphones. Route each strip to A1. |

The default routing sends every strip to A1 (headphones) and sends HW 1 (your mic) and PLAYER to B1 (the stream/call), which is the usual streaming setup.

## Soundboard and music

- Click an empty pad to assign an MP3, WAV, FLAC, OGG or M4A file; click a filled pad to play it; right-click to reassign or clear.
- Pads may overlap. Ctrl+Alt+1–9 fires them even when a game has focus.
- "Load…" in the Music section plays a track; loop and the seek bar work as expected.
- Because PLAYER is a normal strip, its routing buttons decide who hears it: A1 for you, B1 for viewers or the call.

Files are decoded fully into memory when assigned, so a 5-minute track uses roughly 110 MB while loaded.

## Recording

Pick a bus in the top bar and press "● Rec". The file is 32-bit float WAV at 48 kHz.

## Developing on macOS or Linux

The same source builds and runs there (CoreAudio / ALSA through cpal), which is how it is tested. Virtual cables on macOS are BlackHole or Loopback; on Linux, PipeWire loopbacks.

```bash
cargo test
cargo run
```

`cargo check --target x86_64-pc-windows-msvc` type-checks the Windows code paths from any OS once the target is added with `rustup target add x86_64-pc-windows-msvc`.

## Known limits

- Each device runs on its own clock. Over a long session the engine may drop or repeat one 10 ms block occasionally on a device that drifts; the 60 ms ring buffer hides ordinary jitter.
- Exclusive-mode / ASIO devices are not used; everything goes through WASAPI shared mode.
- Files are decoded on the UI thread, so assigning a long track pauses the window for a second.
