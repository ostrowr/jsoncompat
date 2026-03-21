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
  <h1>Escaping Version Skew: Formalizing Compatibility in a World of Partial Rollouts</h1>
  <p>Robbie Ostrow, Member of Technical Staff, OpenAI</p>
  <p>SRECon Americas 2026</p>
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

<h1 class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</h1>

<!--
Set up the joke like you are about to give the wrong kind of operational advice.
The reality underneath it: systems are changing all the time, and we cannot roll
new changes in an instant.
-->

---
layout: center
---

<div class="rollout-joke-stack">
  <h1 class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</h1>
  <h2 class="rollout-joke-punchline">give up</h2>
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
  <div class="law-card success">
    <h3>Great for the wire</h3>
    <p>Less constraining rules preserve compatibility across versions.</p>
  </div>
  <div class="law-card failure">
    <h3>Terrible for logic</h3>
    <p>The point of types is to constrain the states your system can be in.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">A schema that can represent almost anything will eventually represent your next outage.</p>
</div>

<!--
Protos are not expressive enough for the job I care about here.
If your contract is weaker than your business logic, you moved the risk, you did not remove it.
-->

---

<div class="deck-kicker">What to do instead</div>

# Define the boundary as strictly as possible

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
    <h3>Say the primitive type</h3>
    <p>Integer, not “number-like thing we hope is fine.”</p>

    <h3 class="mt-6">Say the invariant</h3>
    <p>If the rule is <code>integer &lt; 5</code>, put it in the contract.</p>

    <h3 class="mt-6">Make nonsense unrepresentable</h3>
    <p>Every forbidden state you encode is one less rollout edge case to reason about by hand.</p>
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
    <h3>Not staged rollouts</h3>
    <p>Rollouts expose problems. They do not define safety.</p>
  </div>
  <div class="law-card accent">
    <h3>Not code review folklore</h3>
    <p>I work with a lot of smart people. No one is careful enough to catch every breaking change by inspection.</p>
  </div>
  <div class="law-card accent">
    <h3>Use a standard</h3>
    <p>Write down what “compatible” means and let a tool be rude consistently.</p>
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
    <p>Schema comparison for the 99% of cases where the rules are tractable.</p>
  </div>
  <div class="law-card success product-card">
    <h3>Fuzzing</h3>
    <p>Generate counterexamples where the static argument runs out of road.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">Move “is this safe?” out of human intuition and into machinery.</p>
</div>

<!--
Introduce the tool in one sentence.
Static analysis first because it is fast and precise when it works.
Fuzzing is the escape hatch for the hard edge cases.
-->

---
layout: center
---

<FuzzingDemo />

<!--
Fuzzing demo beat.
The point is one obvious counterexample that a reviewer can miss:
old world emitted 5, new reader rejects 5 after tightening the bound.
-->

---

<div class="deck-kicker">Final implication</div>

# Do not share runtime types between frontend and backend

<div class="deck-grid-2 mt-10">
  <div class="law-card failure">
    <h3>Shared types feel great</h3>
    <p>One definition, instant reuse, less typing. Very convenient.</p>
  </div>
  <div class="law-card success">
    <h3>Independent evolution is better</h3>
    <p>Generate local types from one contract so each side can move on its own schedule.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">People love sharing types. So give them codegen good enough that they stop needing to.</p>
</div>

<!--
This is the bigger architectural consequence.
Define once at the boundary, generate per side, evolve independently.
-->

---

<div class="deck-kicker">Close</div>

# What I want you to do

<div class="deck-three-laws mt-8">
  <div class="law-card accent">
    <h3>Give up on perfect rollout choreography</h3>
    <p>Assume mixed versions, old packets, old queues, old caches, and old rows.</p>
  </div>
  <div class="law-card accent">
    <h3>Make the contract strict</h3>
    <p>Define primitive types, constraints, and invariants at the boundary.</p>
  </div>
  <div class="law-card accent">
    <h3>Automate the compatibility argument</h3>
    <p>Use static analysis where possible, fuzzing where needed, and codegen to decouple evolution.</p>
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
