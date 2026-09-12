---
format: aep.planning-md/1
id: vision:swarm-builds-itself
kind: vision
status: draft
title: A swarm extends itself, visibly
revision: 1
---
## The objective

A swarm starts bare — one coordinator, one goal — and builds what it needs. The kernel is only what
a swarm cannot build for itself, and everything else is a Box the swarm draws and a Connection it
wires.

Today a swarm can pursue a goal, run a real coordinator, send mail and be watched. What it cannot do
is **act on anything but files**. A box is declared, drawn and connected, and nothing executes one;
a UI box is a kind in an enum and renders nothing; an agent it spawns is a record that never runs.

The objective this collection serves: **a swarm's coordinator can extend the swarm itself** — add a
tool, wire it up, put a panel on the canvas, hand work to another agent — and a person watching can
see it happen and stop it.
