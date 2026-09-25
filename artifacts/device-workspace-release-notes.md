# 🚀 Device workspace release notes

## Summary

StreamMix now separates device setup from mixing, making every physical and virtual endpoint easier to find and configure. The mixer remains intact for live control, while device setup provides ordered, contextual routing guidance.

## Changes

- Added **Mixer** and **Devices** navigation in the top bar.
- Added a scrollable, grouped Devices workspace for all inputs and outputs.
- Put the essential first-run flow first: microphone, headphones/speakers, then stream/call virtual output.
- Added setup progress, assignment/live states, and virtual-cable direction instructions.
- Kept device changes immediate and preserved the existing audio stream setters.

## Repository and branch

| Repository | Branch | Base | PR |
| --- | --- | --- | --- |
| StreamMix | `main` | `main` | not opened |

## Database and config

None.

## Promotion / verification

1. Build the release binary with `/Users/pjdehonor/.cargo/bin/cargo build --release`.
2. Launch StreamMix and open **Devices**.
3. Confirm the first three cards configure mic, headphones/speakers, and virtual stream output in order.
4. Confirm optional hardware and virtual cards are grouped and scrollable, and that a selected endpoint updates its `ASSIGNED`/`LIVE` state.
5. Switch back to **Mixer** and confirm all existing strips, buses, faders, routing, recording, and Player controls remain available.

## Rollback

Revert the change to `src/ui/app.rs`.

## Known gaps

Native-window interaction was not captured automatically in this environment; automated Rust tests, strict linting, compilation, release build, code review, and UI-consistency review all passed.
