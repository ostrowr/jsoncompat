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

<div class="deck-kicker">Incident</div>

# A mixed fleet shared one cache

<IncidentSketch />

<!--
Tell the concrete auth-cache incident, but keep it anonymized on the slide:
- Requests started failing because cache reads raised a parse error.
- The write path changed the cache format from a raw service response body to a
  wrapped object with metadata and payload.
- During deployment, newer pods wrote the new format while older pods still read
  from the same cache and failed to parse it.

Then say:
"It's kind of shocking to me that we, as an industry, haven't solved this problem yet."

That line sets up the natural audience response: "just use protos."
-->

---
layout: center
---

<div class="incident-twist-slide">
  <div class="deck-kicker">Same incident</div>
  <h1>Rollback increased errors</h1>
  <p class="deck-quote mt-8">Old readers came back while bad cached data was still alive.</p>
  <p class="deck-lead deck-muted mt-8">Waiting for the rollout to finish would have caused fewer errors than rolling back.</p>
</div>

<!--
This is the part that makes the system interaction feel genuinely hard:
- As deployment progressed, errors rose because old pods were still present and
  more new-format entries were being written.
- Then errors fell as old pods disappeared.
- Rolling back increased errors again by reintroducing old readers to the cache.
- Eventually the failures stopped only after the bad cache entries expired.
- The Datadog "Errors by Version" chart made this visible after the fact.

The lesson is not "never roll back". It is that rollout safety depends on the
interaction among code versions, shared state, cache TTL, and timing.
- The moment you have persistence, canaries get much weaker as a protection:
  data written by a canary can infect everywhere else.
-->

---

<div class="deck-kicker">Counterargument</div>

# Protos solve transport, not state space

<div class="one-figure-slide mt-10">
  <p class="deck-quote">Wire compatibility can still admit states your logic cannot handle.</p>
  <div class="deck-grid-2 mt-10">
    <div class="law-card failure">
      <h3>On the wire</h3>
      <p>Weak contracts are flexible.</p>
    </div>
    <div class="law-card failure">
      <h3>In the app</h3>
      <p>Weak contracts leak invalid states.</p>
    </div>
  </div>
</div>

<!--
Protos are not expressive enough for the job I care about here.
If your contract is weaker than your business logic, you moved the risk, you did not remove it.
-->

---

<div class="deck-kicker">Boundary</div>

# Parseable is not enough

<div class="one-figure-slide mt-10">
  <p class="deck-quote">Grammar defines shape. Validation defines state.</p>
  <div class="deck-grid-2 mt-10">
    <div class="law-card">
      <h3>Grammar</h3>
      <p>What can be decoded.</p>
    </div>
    <div class="law-card success">
      <h3>Validation</h3>
      <p>What your system is willing to accept.</p>
    </div>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">If the logic depends on the rule, the rule belongs at the boundary.</p>
</div>

<!--
If the room is schema-nerdy, this is where to mention grammar-based versus
rule-based schemas. Otherwise keep it in plain language.

Protovalidate is a good example of a validation layer on top of protobuf:
- It gives you a place to express stronger semantic rules than the grammar.
- That is good, and I would still want it.
- But changing those rules is still a compatibility change under skew.
- A tighter validator is reader narrows. A looser one is reader widens.
- If writers start emitting values that old validators reject, the same rollout
  problem appears again.

As far as I know, Protovalidate does not give you a backward/forward rollout
semantics story for evolving the validation rules themselves, so the same
lessons from this talk apply there too.
-->

---

<div class="deck-kicker">Mental model</div>

# Compatibility is about sets of states

<div class="compat-matrix mt-8">
  <div class="compat-axis compat-axis-top">Reader</div>
  <div class="compat-axis compat-axis-left">Writer</div>

  <div class="compat-cell success">
    <div class="compat-cell-title">Narrows</div>
    <div class="compat-cell-body">Usually safe.</div>
  </div>
  <div class="compat-cell danger">
    <div class="compat-cell-title">Widens</div>
    <div class="compat-cell-body">Old readers may reject new values.</div>
  </div>
  <div class="compat-cell success">
    <div class="compat-cell-title">Widens</div>
    <div class="compat-cell-body">Usually safe.</div>
  </div>
  <div class="compat-cell danger">
    <div class="compat-cell-title">Narrows</div>
    <div class="compat-cell-body">Old data may be rejected.</div>
  </div>
</div>

<div class="deck-callout compat-takeaway mt-8">
  <p class="deck-quote">Backward and forward describe parse direction. Rollout safety also depends on emission, overlap, and time.</p>
</div>

<!--
This is the compact mental model I want people to leave with:
- A schema change changes a set of values.
- Writer narrows: usually safe, because it emits fewer states.
- Writer widens: dangerous under skew, because old readers may see values they
  cannot parse.
- Reader widens: usually safe, because it accepts more historical states.
- Reader narrows: dangerous under skew, because old data or old writers may
  still exist.

Prior art to mention:
- Avro explicitly separates the writer's schema from the reader's schema and
  defines schema resolution between them.
- Confluent Schema Registry ties compatibility modes to upgrade order:
  BACKWARD means upgrade consumers before producing new events; FORWARD means
  upgrade producers first and drain old data before upgrading consumers; FULL
  allows independent upgrades.
- That is useful when upgrade order is a real control surface. In my world,
  partial rollouts, retries, caches, queues, and rollback mean order is often
  not guaranteed, so I need stronger constraints and tooling than pairwise
  backward/forward labels.
-->

---

<div class="deck-kicker">What to do instead</div>

# Write the boundary as strictly as the logic

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
      <div class="boundary-point-title">Guarantee</div>
      <div class="boundary-point-body">Only schema invariants are guaranteed. Reject bad input at the boundary.</div>
    </div>
  </div>
</div>

<!--
This is the constructive turn.
Explicitly define boundaries. Constrain both primitive type and semantic shape.
The stricter the contract, the smaller the mixed-version state space.
Systems cannot assume anything not represented in the schema. Push as much as
possible into that layer so invalid input gets rejected gracefully at the
boundary, before application logic has to handle it at all.
-->

---
class: demo-full-bleed
---

<SimulatorDeck
  mode="transition"
  start-state-id="s6"
  :sequence="['s7', 's8', 's9']"
  :step-delay-ms="1600"
  :autoplay="false"
  :pause-at-end="true"
  :emit-rate-per-sec="1.1"
  :packet-speed-px-per-sec="78"
  :initial-packet-count="3"
  :initial-packet-spacing-px="220"
  :minimum-packet-gap-px="220"
  height="72vh"
  :layout-scale="0.5"
  :bare="true"
/>

<!--
This is the constructive rollout pattern:
- First deploy a reader union that can parse both v4 and v5.
- Then deploy the writer change to start emitting v5.
- Finally remove v4 support once the old data tail is gone.
This is the answer to "what do I do instead of letting things break?"
-->

---

<div class="deck-kicker">Process</div>

# Check compatibility with tools, not memory

<div class="deck-grid-2 mt-10">
  <div class="law-card success product-card">
    <h3>Static analysis</h3>
    <p>Prove the common cases before deploy.</p>
  </div>
  <div class="law-card success product-card">
    <h3>Fuzzing</h3>
    <p>Search for counterexamples where proofs run out.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">Do not ask reviewers to simulate a distributed system in their head.</p>
</div>

<!--
This is the practical advice slide.
The failure mode is not intelligence, it is that humans are bad at mixed-version reasoning.
-->

---
layout: center
---

<div class="demo-setup-line">Here’s the kind of break code review misses.</div>

<!--
Set up the live demo as proof, not product marketing.
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
layout: center
---

<div class="pairing-takeaway">
  <div class="deck-kicker">Tooling</div>
  <p class="deck-quote mt-8">Static analysis for the common case.</p>
  <p class="deck-quote mt-2">Fuzzing for the rest.</p>
</div>

<!--
Introduce the tool in one sentence.
Static analysis first because it is fast and precise when it works.
Fuzzing is the escape hatch for the hard edge cases.
TODO: mention that we mark wire types with a decorator so compatibility checking
is attached to the boundary type itself.
-->

---

<div class="deck-kicker">Tooling</div>

# Writers only emit what readers can parse

<div class="deck-grid-3 mt-10">
  <div class="law-card success">
    <h3>Writer code</h3>
    <p>Only writes the generated <code>Reader</code> type.</p>
  </div>
  <div class="law-card accent">
    <h3>Breaking change</h3>
    <p>Codegen expands the writer type to an explicit union.</p>
  </div>
  <div class="law-card success">
    <h3>CI</h3>
    <p>Rejects any write shape that deployed readers cannot parse.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">If old readers cannot parse it, the writer change is forbidden.</p>
</div>

<!--
This is the enforcement model:
- Application writers should only write values in the generated reader contract,
  not an ad hoc local type.
- If a schema change would break partial rollout safety, codegen should force an
  explicit union into the generated writer-side type.
- CI should make that impossible to ignore by rejecting writes that are not
  accepted by the reader population you need to support.
- This does not mean every change becomes legal. Changes that introduce writer
  states unreadable by still-deployed readers are impermissible and should be
  blocked outright.
-->

---

<div class="deck-kicker">Final implication</div>

# One contract. Two generated local types.

<div class="deck-grid-3 mt-10">
  <div class="law-card success">
    <h3>Serializer</h3>
    <p>Emit less.</p>
  </div>
  <div class="law-card success">
    <h3>Deserializer</h3>
    <p>Accept more.</p>
  </div>
  <div class="law-card failure">
    <h3>One shared runtime type</h3>
    <p>Becomes optional soup.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">If old readers never go away, keep a discriminated union of past types. Do not weaken one type until it means everything.</p>
</div>

<!--
Points to hit:
- Serializer and deserializer compatibility are asymmetric during a partial rollout.
- A serializer wants a narrower output set; a deserializer wants a wider accepted input set.
- If one runtime type serves both, you usually end up with the union of rollout-era compromises, not the real domain model.
- That weakens invariants exactly where you wanted types to protect you.
- Queues, caches, and stored rows keep the old serialized shape alive after deploy, so reader compatibility is a long tail.
- Define one strict boundary contract, then generate separate local types for each side.
- Even if you can never fully retire an old reader shape, an explicit
  discriminated union of historical variants is still better than one weak type
  that tries to express every era at once.
-->

---

<div class="deck-kicker">Close</div>

# Assume skew. Constrain boundaries. Automate checks.

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

<div class="thanks-slide">
  <div class="thanks-title">Thank you</div>
</div>
