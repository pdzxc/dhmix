# 🔍 Device workspace evidence

## Outcome

The device configuration flow is now separate from the dense mixer surface. It presents the required microphone, listening device, and stream/call virtual output in order, then groups every remaining hardware input, virtual input, Player, hardware output, and virtual output in a scrollable workspace.

## Observable: device navigation

| Before | After |
| --- | --- |
| Device selectors were embedded in fourteen narrow channel and bus columns. Setup guidance disappeared after any single selector was set. | The top bar exposes **Mixer** and **Devices** workspaces. The Devices workspace begins with the three required choices and a live `0–3` progress indicator, then lists optional hardware/virtual sections. |
| Hardware and virtual endpoints were differentiated only by their column titles. | Each card has a purpose, `LIVE` / `ASSIGNED` / `UNASSIGNED` state, type-appropriate color, and cable-direction guidance where applicable. |

## Evidence capture notes

- Baseline visual: [the repository’s prior mixer screenshot](../screenshot-macos.png). It documents the original fourteen-column arrangement, but was not captured during this change; the original live application was not launched before edits because `cargo` was absent from the shell PATH.
- After-state visual capture gap: the native executable builds and `cargo run` starts, but it is not exposed as an accessible application surface in this environment, so an automated native-window screenshot could not be taken. The after state was verified through code review, UI-consistency review, strict linting, and the Rust test suite.

## Changed file

- `src/ui/app.rs`

## Verification

- Code review: **APPROVE**
- UI consistency: **CONSISTENT**
- Test report: [device workspace test report](device-workspace-test-report.md)
