---
title: Why we built SKYCOT
summary: AI is becoming a teammate, not a tool. SKYCOT is the platform we wished existed when we started trying to delegate real work.
date: 2026-05-15
author: Mark Hallam
---

{/* REVIEW-REQUIRED: prose */}

The first time I tried to delegate a real piece of work to an AI, it took me longer to write the prompt than it would have taken to just do the thing.

That's the gap. Every AI tool I've used for the last three years sits on the wrong side of it. The interface is a chat box. The output is a draft of something I have to copy-paste. The state of the world is whatever I remembered to tell it five turns ago. And when it gets something wrong, there's nobody to ask why.

When I hired my first remote engineer, none of that was true. I told her what the company was, what the role was, what the team looked like. We onboarded her into a Slack workspace. She read the docs. She asked clarifying questions. When she didn't know how to do something, she went and figured it out — or she asked me, and we agreed on the approach before she spent half a day on it.

That is the loop we are building.

## The shape of the product

SKYCOT generates AI workers from plain-English job descriptions. Not chat sessions. Not single-turn API calls. Workers with persistent identity, memory of past tasks, a Slack handle, an email address, a calendar of meetings they've been in. Hire them, give them the same onboarding you'd give a person, and let them work.

There are four things that make this different from existing AI tools:

1. **Workers stay around.** When you onboard a SKYCOT receptionist on Monday and tell her to greet visitors by name, she still greets visitors by name on Friday — without you re-prompting her.

2. **Workers ask for approval like a human would.** Real teammates don't push code straight to production. They open pull requests. They Slack you before sending a sensitive email. SKYCOT workers do the same.

3. **Workers run on the same tools you do.** Computer-use on a real macOS desktop. Slack with a real handle. MCP for everything else. We're not building a wrapper that runs in a sandbox — we're building a teammate that uses your tools.

4. **Workers learn from feedback.** When you correct a draft, that correction is captured as a skill that propagates to the next task. The worker improves the way a junior employee improves: by being told and remembering.

## The hard part

The hard part is not making the model write good text. The hard part is making the worker *reliable enough to trust* — to give it a real role, with real authority, in a real organization, and not have to babysit it.

That means: a runtime that doesn't crash. Observability so you can see what the worker is doing and why. Repeatable behavior across runs. Tenancy so two workers in two companies don't leak to each other. Audit logs you can show to a compliance team.

These are not glamorous problems. They are the problems that separate "demo" from "product." We have spent the last year on them, and we're going to spend the next year continuing to spend on them.

## Where this goes

The big bet is that the next ten years of work will not be humans pasting prompts into chat boxes. It will be humans giving a new kind of teammate a job to do, watching the first few attempts, correcting the parts that need correcting, and then trusting that teammate the way you'd trust a junior hire after a month.

That's the future we want to live in. That's why we built SKYCOT.
