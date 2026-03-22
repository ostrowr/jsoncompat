---
theme: seriph
layout: default
title: Escaping Version Skew
info: |
  ## Escaping Version Skew

  Compatibility, version skew, and what to do about it when rollouts are never instant.
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

<NetworkHero />

<!--
Pre-talk slide and first beat.
Let it run while people settle. Then start by saying this is the fantasy:
a big distributed system, data moving around, destinations lighting up green,
everything looks legible from far away.
-->

---
class: demo-full-bleed
---

<NetworkHero :red-receive-every="10" :title-layout="true" :hidden-node-ids="['router', 'db']">
  <div class="hero-title-copy">
    <div class="hero-talk-title">Escaping Version Skew: Formalizing Compatibility in a World of Partial Rollouts</div>
    <div class="hero-talk-meta">Robbie Ostrow, Member of Technical Staff, OpenAI</div>
    <div class="hero-talk-event">SRECon Americas 2026</div>
  </div>
</NetworkHero>

<!--
Same system, but now the comforting green "arrived" signal is not what we get.
Use the title here, next to the failing nodes, after the cold open has already
established the motion.
-->

---
class: demo-full-bleed
---

<div class="zoom-bridge-shell">
  <SimulatorDeck
    mode="steady"
    start-state-id="s1"
    :emit-rate-per-sec="1.15"
    :packet-speed-px-per-sec="78"
    :initial-packet-count="2"
    :initial-packet-spacing-px="220"
    :minimum-packet-gap-px="220"
    height="72vh"
    :layout-scale="0.5"
    :bare="true"
    :show-state-chip="false"
  />
</div>

<!--
Zoom in to one subsystem.
This is the current first slide, now reframed as steady-state operation and
the animated-ish zoomed-in version:
everything in its place, everything working as intended.
-->

---
layout: center
---

<div class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</div>

<!--
Set up the joke like you are about to give the wrong kind of operational advice.
The reality underneath it: systems are changing all the time, and we cannot roll
new changes in an instant.
-->

---
layout: center
---

<div class="rollout-joke-stack">
  <div class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</div>
  <div class="rollout-joke-punchline">give up</div>
</div>

<!--
Land the punchline plainly.
Not give up on correctness; give up on pretending perfect choreography across
mixed versions is a strategy.
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
  :emit-rate-per-sec="1.3"
  :packet-speed-px-per-sec="78"
  :initial-packet-count="4"
  :initial-packet-spacing-px="220"
  :minimum-packet-gap-px="220"
  height="72vh"
  :layout-scale="0.5"
  :bare="true"
/>

<!--
Use this as the minimum mechanics demo, not the whole talk.
Old packets are still in flight while new code is already live.
One tiny diff becomes two different compatibility questions depending on direction.
-->

---

<div class="deck-kicker">Counterargument</div>

# “Just use protos”

<div class="deck-grid-2 mt-10">
  <div class="law-card failure">
    <h3>On the wire</h3>
    <p>Compatibility likes weak contracts.</p>
  </div>
  <div class="law-card failure">
    <h3>In the app</h3>
    <p>Correctness needs strong ones.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">A type that permits everything protects nothing.</p>
</div>

<!--
Protos are not expressive enough for the job I care about here.
If your contract is weaker than your business logic, you moved the risk, you did not remove it.
-->

---

<div class="deck-kicker">What to do instead</div>

# Write the boundary like law

<div class="deck-grid-2 mt-8">
  <div class="deck-schema-box">

```json
{
  "type": "object",
  "properties": {
    "retries": {
      "type": "integer",
      "minimum": 0,
      "exclusiveMaximum": 5
    },
    "mode": {
      "enum": ["fast", "safe"]
    }
  },
  "required": ["retries", "mode"]
}
```

  </div>
  <div class="fact-card boundary-card">
    <div class="boundary-point">
      <div class="boundary-point-title">Primitive</div>
      <div class="boundary-point-body"><code>integer</code>, not “number-ish”.</div>
    </div>
    <div class="boundary-point">
      <div class="boundary-point-title">Invariant</div>
      <div class="boundary-point-body">If the rule is <code>&lt; 5</code>, write <code>&lt; 5</code>.</div>
    </div>
    <div class="boundary-point">
      <div class="boundary-point-title">State space</div>
      <div class="boundary-point-body">Every forbidden state you encode is one less mixed-version edge case.</div>
    </div>
  </div>
</div>

<!--
This is the constructive turn.
Explicitly define boundaries. Constrain both primitive type and semantic shape.
The stricter the contract, the smaller the mixed-version state space.
-->

---

<div class="deck-kicker">Process</div>

# Evolve with tooling, not vibes

<div class="deck-grid-3 mt-10">
  <div class="law-card accent">
    <h3>Rollouts find bugs</h3>
    <p>They do not define safety.</p>
  </div>
  <div class="law-card accent">
    <h3>Review misses edges</h3>
    <p>Smart people still lose to mixed-version reasoning.</p>
  </div>
  <div class="law-card accent">
    <h3>Tools do not get tired</h3>
    <p>Let them be rude consistently.</p>
  </div>
</div>

<!--
This is the practical advice slide.
The failure mode is not intelligence, it is that humans are bad at mixed-version reasoning.
-->

---

<div class="deck-kicker">Product</div>

# `jsoncompat`

<div class="deck-grid-2 mt-10">
  <div class="law-card success product-card">
    <h3>Static analysis</h3>
    <p>Prove the easy 99%.</p>
  </div>
  <div class="law-card success product-card">
    <h3>Fuzzing</h3>
    <p>Hunt counterexamples in the rest.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">Do not ask reviewers to simulate a distributed system in their head.</p>
</div>

<!--
Introduce the tool in one sentence.
Static analysis first because it is fast and precise when it works.
Fuzzing is the escape hatch for the hard edge cases.
-->

---
class: demo-full-bleed
---

<FuzzerEmbed />

<!--
Fuzzing demo beat.
Use the real jsoncompat.com fuzzer instead of a mock. The point is one obvious
counterexample that a reviewer can miss: old world emitted 5, new reader
rejects 5 after tightening the bound.
-->

---

<div class="deck-kicker">Final implication</div>

# Shared runtime types erase direction

<div class="deck-grid-3 mt-10">
  <div class="law-card failure">
    <h3>Serializer</h3>
    <p>Emit less.</p>
  </div>
  <div class="law-card failure">
    <h3>Deserializer</h3>
    <p>Accept more.</p>
  </div>
  <div class="law-card success">
    <h3>Shared type</h3>
    <p>Becomes optional soup.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">One contract. Two generated local types.</p>
</div>

<!--
Points to hit:
- Serializer and deserializer compatibility are asymmetric during a partial rollout.
- A serializer wants a narrower output set; a deserializer wants a wider accepted input set.
- If one runtime type serves both, you usually end up with the union of rollout-era compromises, not the real domain model.
- That weakens invariants exactly where you wanted types to protect you.
- Queues, caches, and stored rows keep the old serialized shape alive after deploy, so reader compatibility is a long tail.
- Define one strict boundary contract, then generate separate local types for each side.
-->

---

<div class="deck-kicker">Close</div>

# What I want you to do

<div class="deck-three-laws mt-8">
  <div class="law-card accent">
    <h3>Assume skew</h3>
    <p>Old packets, old queues, old caches, old rows.</p>
  </div>
  <div class="law-card accent">
    <h3>Constrain the boundary</h3>
    <p>Primitive types, constraints, invariants.</p>
  </div>
  <div class="law-card accent">
    <h3>Automate the proof</h3>
    <p>Static analysis, fuzzing, codegen.</p>
  </div>
</div>

<!--
End on advice, not mechanics.
This is the compact version of the whole talk.
-->

---
layout: center
---

<div class="deck-panel px-10 py-12 max-w-4xl mx-auto">
  <div class="deck-kicker">Closing line</div>
  <p class="deck-quote mt-6">
    Compatibility is not the art of being vague.
  </p>
  <p class="deck-quote mt-2">It is what strictness buys you.</p>
</div>

---
layout: center
---

<div class="thanks-slide">
  <div class="deck-kicker">Questions</div>
  <div class="thanks-title">Thank you</div>
  <p class="deck-lead mt-4">Questions for the compatibility tribunal.</p>
</div>
