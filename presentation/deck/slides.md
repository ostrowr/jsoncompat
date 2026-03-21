---
theme: seriph
title: Escaping Version Skew
info: |
  ## Escaping Version Skew

  Compatibility, version skew, and why strict invariants make rollouts less stupid.
class: demo-full-bleed
colorSchema: light
routerMode: hash
aspectRatio: 16/9
canvasWidth: 960
fonts:
  sans: IBM Plex Sans
  mono: IBM Plex Mono
  serif: Source Serif 4
mdc: true
drawings:
  persist: false
---

<SimulatorDeck
  mode="steady"
  start-state-id="s1"
  :emit-rate-per-sec="0.76"
  :packet-speed-px-per-sec="78"
  :initial-packet-count="1"
  :initial-packet-spacing-px="320"
  :minimum-packet-gap-px="280"
  height="72vh"
  :layout-scale="0.5"
  :bare="true"
  :show-state-chip="false"
/>

<!--
Pre-talk slide.
Leave this up before you start.
-->

---

<div class="deck-kicker">SRECon Americas 2026 • Robbie Ostrow • OpenAI</div>

# Escaping Version Skew

<p class="deck-lead max-w-3xl">
Your schema changes in one commit. Production changes in installments.
</p>

<div class="deck-grid-2 mt-10">
  <div class="quote-card">
    <h3>The problem</h3>
    <p>Partial rollouts turn “is this a safe change?” into a distributed-systems question.</p>
  </div>
  <div class="quote-card">
    <h3>The thesis</h3>
    <p>If your invariants are precise enough, compatibility stops being an interpretive art.</p>
  </div>
</div>

<!--
Open cleanly.
No throat clearing.
-->

---

<div class="deck-kicker">Operational reality</div>

# Production is full of old packets

<div class="deck-grid-2 mt-8">
  <div class="fact-card">
    <h3>Requests are still in flight</h3>
    <p>The old writer can still be talking while the new reader is already live.</p>
  </div>
  <div class="fact-card">
    <h3>Queues preserve history</h3>
    <p>Backlog is version skew with a product manager.</p>
  </div>
  <div class="fact-card">
    <h3>Caches outlive deploys</h3>
    <p>Yesterday’s payload can still be today’s incident.</p>
  </div>
  <div class="fact-card">
    <h3>Stored JSON never resigned</h3>
    <p>The database is how your old schema remains a voting member.</p>
  </div>
</div>

<!--
This is the setup for why directionality matters.
-->

---

<div class="deck-kicker">Compatibility law</div>

# Breaking for whom?

<div class="deck-grid-2 mt-10">
  <div class="law-card success">
    <h3>Writer-compat</h3>
    <p>The new writer must stay inside the old reader’s idea of valid data.</p>
    <p class="deck-micro mt-4"><code>new ⊆ old</code></p>
  </div>
  <div class="law-card failure">
    <h3>Reader-compat</h3>
    <p>The new reader must still accept everything the old world already emitted.</p>
    <p class="deck-micro mt-4"><code>old ⊆ new</code></p>
  </div>
</div>

<!--
Do not linger on terminology.
Say writer and reader more than serializer and deserializer.
-->

---

<div class="deck-kicker">Tiny diff</div>

# One diff, two press releases

<div class="deck-grid-2 mt-8">
  <div class="deck-schema-box">

```json
// old
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" }
  }
}
```

  </div>
  <div class="deck-schema-box">

```json
// new
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" }
  },
  "required": ["id"]
}
```

  </div>
</div>

<div class="deck-grid-2 mt-8">
  <div class="law-card success">
    <h3>The writer says: stricter</h3>
    <p>The new writer emits fewer nonsense objects. Great.</p>
  </div>
  <div class="law-card failure">
    <h3>The reader says: absolutely not</h3>
    <p>The new reader just criminalized old data that omitted <code>id</code>.</p>
  </div>
</div>

<!--
This is the first real "wat" moment.
-->

---
class: demo-full-bleed
---

<SimulatorDeck
  mode="transition"
  start-state-id="s1"
  :sequence="['s2', 's3']"
  :step-delay-ms="1600"
  :autoplay="false"
  :pause-at-end="true"
  :emit-rate-per-sec="0.78"
  :packet-speed-px-per-sec="78"
  :initial-packet-count="3"
  :initial-packet-spacing-px="240"
  :minimum-packet-gap-px="220"
  height="72vh"
  :layout-scale="0.5"
  :bare="true"
/>

<!--
State map:
- s1 writer v1 reader v1
- s2 writer v1 reader v2
- s3 writer v2 reader v2
-->

---

<div class="deck-kicker">Translation</div>

# A deploy is not a rewrite pass over reality

<div class="deck-grid-3 mt-8">
  <div class="beat-card">
    <h3>The wire keeps receipts</h3>
    <p>Anything already emitted keeps the old shape.</p>
  </div>
  <div class="beat-card">
    <h3>Readers inherit history</h3>
    <p>Your new code has to coexist with packets it did not negotiate.</p>
  </div>
  <div class="beat-card accent">
    <h3>This is a contract problem</h3>
    <p>Not a “please coordinate the deploy” problem.</p>
  </div>
</div>

---
class: demo-full-bleed
---

<SimulatorDeck
  mode="transition"
  start-state-id="s3"
  :sequence="['s4', 's5']"
  :step-delay-ms="1750"
  :autoplay="false"
  :pause-at-end="true"
  :emit-rate-per-sec="0.78"
  :packet-speed-px-per-sec="78"
  :initial-packet-count="3"
  :initial-packet-spacing-px="240"
  :minimum-packet-gap-px="220"
  height="72vh"
  :layout-scale="0.5"
  :bare="true"
/>

---

<div class="deck-kicker">Common mistake</div>

# “Just make it optional” is how types become vibes

<div class="deck-grid-2 mt-8">
  <div class="law-card failure">
    <h3>Short-term benefit</h3>
    <p>Fewer immediate decoder explosions.</p>
  </div>
  <div class="law-card accent">
    <h3>Long-term bill</h3>
    <p>You stop being able to say, with a straight face, what your data means.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">
    Loose schemas are very flexible. They can represent almost anything, including your next outage.
  </p>
</div>

---

<div class="deck-kicker">The contrarian bit</div>

# Strictness is what makes automation possible

<div class="deck-grid-3 mt-8">
  <div class="law-card success">
    <h3>Say what must be true</h3>
    <p>If a field is required, require it. If it is one of three values, say so.</p>
  </div>
  <div class="law-card success">
    <h3>Ban nonsense early</h3>
    <p>The smaller the valid state space, the less garbage you reason about during rollout.</p>
  </div>
  <div class="law-card success">
    <h3>Let the tool be rude</h3>
    <p>Humans are terrible at mixed-version reasoning, especially when the diff looks small.</p>
  </div>
</div>

---

<div class="deck-kicker">Case study</div>

# `jsoncompat` is a hall monitor for schema changes

<div class="deck-grid-2 mt-8">
  <div class="deck-schema-box">

```rust
pub fn check_compat(old: &SchemaNode, new: &SchemaNode, role: Role) -> bool {
    match role {
        Role::Serializer => is_subschema_of(new, old),
        Role::Deserializer => is_subschema_of(old, new),
        Role::Both => is_subschema_of(new, old) && is_subschema_of(old, new),
    }
}
```

  </div>
  <div class="fact-card">
    <h3>Why this matters</h3>
    <p>The decision stops living in code review folklore and starts living in a tool with an actual standard.</p>
    <p class="mt-4">Once “safe” has a definition, the computer can ruin your day consistently.</p>
  </div>
</div>

---
class: demo-full-bleed
---

<SimulatorDeck
  mode="transition"
  start-state-id="s6"
  :sequence="['s7', 's8', 's9']"
  :step-delay-ms="1450"
  :autoplay="false"
  :pause-at-end="true"
  :emit-rate-per-sec="0.98"
  :packet-speed-px-per-sec="108"
  :initial-packet-count="4"
  :initial-packet-spacing-px="176"
  height="74vh"
  :layout-scale="0.54"
  :bare="true"
/>

---

<div class="deck-kicker">Close</div>

# Three rules I actually use

<div class="deck-three-laws mt-8">
  <div class="law-card accent">
    <h3>Name the direction</h3>
    <p>Ask who changed, who still emits data, and who still has to read it.</p>
  </div>
  <div class="law-card accent">
    <h3>Assume mixed versions</h3>
    <p>Packets, queues, caches, and databases all outlive the tidy line on your rollout chart.</p>
  </div>
  <div class="law-card accent">
    <h3>Be strict on purpose</h3>
    <p>Make impossible states unrepresentable, then automate the argument about change.</p>
  </div>
</div>

---
layout: center
---

<div class="deck-panel px-10 py-12 max-w-4xl mx-auto">
  <div class="deck-kicker">Closing line</div>
  <p class="deck-quote mt-6">
    Compatibility is not the art of being vague.
    <br>
    It is what strictness buys you.
  </p>
</div>

---
layout: center
---

<div class="text-center">
  <div class="deck-kicker">Questions</div>
  <h1 class="mt-6">Thank you</h1>
  <p class="deck-lead mt-4">I will now take questions from the compatibility tribunal.</p>
</div>
