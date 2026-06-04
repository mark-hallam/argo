---
title: Computer-use on macOS — why we made it the default
summary: How SKYCOT's macOS computer-use executor works, what made us pick a real desktop over a sandbox, and the engineering trade-offs that came with that choice.
date: 2026-05-14
author: SKYCOT Engineering
---

{/* REVIEW-REQUIRED: prose */}

Most AI agent platforms run their computer-use executor in a sandbox. The model sees a screenshot, generates an action, and the action runs in a remote VM that gets reset between sessions. This is the safe, scalable default — but it's also the wrong abstraction for what most users actually want.

What users actually want is for the worker to be able to use *their* apps. The Salesforce instance with their data in it. The Figma file they're editing. The accounting software that only runs on the operator's laptop. A sandboxed VM can't reach any of those without a great deal of plumbing.

So we took a different bet. For SKYCOT v1, the primary computer-use executor runs on real macOS — the same desktop the operator is using.

## How it works

The `argo-executor-computer-use-macos` crate wraps three macOS APIs:

- **CoreGraphics** for screen capture. We sample at 1 Hz when idle and 4 Hz during active action sequences.
- **Accessibility (AX)** for UI element introspection. This is what lets the model target a specific button by its accessible name rather than guessing pixel coordinates.
- **CGEvent** for mouse and keyboard input. Synthetic events with sourcePID set to a sentinel, so they show up in security audit logs as agent-driven.

The model sees a hybrid representation: a screenshot for visual grounding, plus an accessibility tree for semantic targeting. Most actions resolve through the AX tree (faster, more reliable). Pixel coordinates are a fallback for apps that don't expose AX well.

## What we learned

A few surprises came out of building this.

**AX support is a long tail.** Native Cocoa apps expose AX cleanly. Electron apps mostly do. Anything built on a non-native framework — Flutter desktop, certain Java apps, some games — exposes essentially nothing. The model has to fall back to pure vision in those cases, and reliability drops.

**Frame rate matters less than you'd think.** We initially sampled at 30 Hz to feel "real-time." It turned out the model couldn't usefully consume frames that fast — most of them were redundant. Dropping to 4 Hz cut bandwidth by 7.5× with no measurable accuracy loss.

**Permission prompts are the real UX boundary.** macOS asks for screen-recording and accessibility permissions per-app. We had to design an onboarding flow that walks the operator through granting these to the SKYCOT binary specifically — and then a separate flow for re-granting after macOS updates revoke them.

**Linux and Windows are deferred, not abandoned.** Both executors exist in the codebase. They work. They're just not hardened to v1 quality. We made a call: ship one OS well rather than three OSes shakily.

## Trade-offs

Running on the operator's real desktop has obvious downsides:

- The worker can only act when the operator's machine is on.
- If the operator is using the machine, the worker has to either wait or share the screen (which is jarring).
- Security boundaries are looser than a clean VM.

We mitigate these in v1.x with optional cloud-hosted desktops for users who want them. But the demo path — the thing we show on the home page — is: install SKYCOT, hire a worker, watch it use your real apps. That's the right experience to optimize for first.

## What's next

The next thing on this track is action recording. We want to capture an operator demonstrating a workflow once, then have the worker reproduce it autonomously — with the AX tree as the durable abstraction so the recording survives small UI changes.

If you want to try the macOS executor, install the latest SKYCOT release and hire a worker with the `computer-use` capability enabled. The first action will prompt you for permissions; everything after that should "just work."
