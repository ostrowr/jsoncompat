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

<NetworkHero :red-receive-every="10" :title-layout="true" :hidden-node-ids="['router', 'db']">
  <div class="hero-title-copy">
    <div class="hero-talk-title">
      <span class="hero-title-line">Escaping Version Skew:</span>
      <span class="hero-title-line hero-title-line-nowrap">Formalizing Compatibility</span>
      <span class="hero-title-line">in a World of Partial Rollouts</span>
    </div>
    <div class="hero-talk-meta">Robbie Ostrow, Member of Technical Staff, OpenAI</div>
    <div class="hero-talk-event">SRECon Americas 2026</div>
  </div>
</NetworkHero>

<!--
Open on the fantasy and the break at once: a system diagram that looks legible
from far away, except some arrivals are already red.
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
  <p class="deck-lead deck-muted mt-8">Waiting for the rollout to finish would have caused fewer errors than rolling back. This failed because one live version wrote a state that another still-live version could not accept.</p>
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
- Speaker note, not slide text: keep using `4` as the tiny recurring edge-value
  villain in the story.
-->

---
layout: center
---

<div class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</div>

<!--
Setup beat for the punchline.
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-word">give up</div>
</div>

<!--
Punchline: not give up on correctness; give up on pretending perfect choreography
across mixed versions is a strategy.
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase">Don't rely on rollout order.</div>
</div>

<!--
Give the first refrain its own slide so it lands as the thesis, not a subtitle.
-->

---
class: demo-full-bleed
---

<div class="simulator-slide-shell">
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
    :show-state-chip="false"
  />
</div>

<!--
Use this as the minimum mechanics demo, not the whole talk.
It starts in the simplified steady-state model people usually reason from, then
advances into broken overlap on keypress.
Old packets are still in flight while new code is already live.
One tiny diff becomes two different compatibility questions depending on direction.
-->

---

<div class="deck-kicker">Boundary</div>

# Parseable is not enough

<div class="one-figure-slide mt-8">
  <p class="deck-quote">Transport compatibility can still admit states your logic cannot handle.</p>
  <div class="deck-grid-2 mt-8">
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

<div class="deck-callout mt-8">
  <p class="deck-quote">If the logic depends on the rule, the rule belongs at the boundary.</p>
</div>

<!--
This merges the "just use protos" counterargument into the boundary point:
parseable is weaker than valid state.

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

<p class="mental-model-subhead">A schema change changes a set of states.</p>

<div class="compat-matrix mt-6">
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
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase">Only the contract is guaranteed.</div>
</div>

<!--
Second refrain as a standalone beat before the constructive slide.
-->

---

<div class="deck-kicker">What to do instead</div>

# Only the contract is guaranteed

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
      <div class="boundary-point-body">If the rule is <code>&lt; 5</code>, write <code>&lt; 5</code>. The edge value <code>4</code> is the one that keeps coming back.</div>
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

Call out the agent angle explicitly:
- Agents are worse than humans at recovering hidden assumptions across abstraction
  boundaries.
- Tight contracts give them a smaller legal state space and a sharper test oracle.
-->

---

<div class="deck-kicker">AI agents</div>

# Strict contracts are better for agents

<div class="deck-grid-3 mt-8 agent-contract-grid">
  <div class="law-card success">
    <h3>Smaller legal state space</h3>
    <p>Fewer ambiguous shapes for an agent to invent, infer, or accidentally depend on.</p>
  </div>
  <div class="law-card success">
    <h3>Hidden assumptions become explicit</h3>
    <p>Put the rule at the boundary so the agent does not have to recover it from prose, examples, or tribal context.</p>
  </div>
  <div class="law-card success">
    <h3>Crisper test oracle</h3>
    <p>A strict contract turns "looks plausible" into pass/fail examples that CI and code review can both enforce.</p>
  </div>
</div>

<div class="deck-callout mt-8">
  <p class="deck-quote">Agentic workflows get safer when the boundary is narrow enough to make bad states impossible, not just unlikely.</p>
</div>

<!--
Make the agent point concrete and engineering-focused:
- Large model callers are especially bad at reconstructing implicit invariants
  from surrounding context.
- Tight contracts reduce the amount of hidden reasoning the agent has to do.
- They also give you a sharper oracle for automated checks and review.
-->

---
class: demo-full-bleed
---

<div class="simulator-slide-shell">
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
    :show-state-chip="false"
  />
</div>

<!--
This is the constructive rollout pattern:
- First deploy a reader union that can parse both v4 and v5.
- Then deploy the writer change to start emitting v5.
- Finally remove v4 support once the old data tail is gone.
This is the answer to "what do I do instead of letting things break?"
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase">Check it mechanically.</div>
</div>

<!--
Third refrain as a standalone beat before the checker/tooling section.
-->

---
class: demo-full-bleed
---

<CheckerEmbed />

<!--
Static compat-check demo beat.
Use the real jsoncompat.com checker instead of the fuzzer. The point is that
this break does not require example search: for the `exclusiveMaximum: 5` to `4`
change, incompatibility is derivable from the old and new contracts directly.
`4` is the witness that explains the failure: old writers can still emit it, and
new readers reject it.
-->

---

<div class="deck-kicker">Tooling</div>

# Writers only emit what readers can parse

<div class="tooling-pipeline mt-6">
  <div class="tooling-step law-card success">
    <div class="tooling-step-label">1. Detect</div>
    <h3>Breaking change?</h3>
    <p>Prove what you can statically. Search for counterexamples when needed.</p>
  </div>

  <div class="tooling-arrow" aria-hidden="true">-></div>

  <div class="tooling-step law-card accent">
    <div class="tooling-step-label">2. Generate</div>
    <h3>Reader and Writer types</h3>
    <p>One schema, two local types. Writer code only emits <code>Reader</code>.</p>
  </div>

  <div class="tooling-arrow" aria-hidden="true">-></div>

  <div class="tooling-step law-card success">
    <div class="tooling-step-label">3. Stamp</div>
    <h3>On writer break, add a tagged branch</h3>
    <div class="tooling-union-stack" aria-label="Reader union stamped with a new version">
      <div class="tooling-union-chip">Reader =</div>
      <div class="tooling-union-chip">v4</div>
      <div class="tooling-union-plus">|</div>
      <div class="tooling-union-chip tooling-union-new">v5</div>
    </div>
  </div>
</div>

<div class="deck-callout mt-8">
  <p class="deck-quote">If old readers cannot parse it, the writer change is forbidden.</p>
</div>

<!--
This is the enforcement model:
- Application writers should only write values in the generated reader contract,
  not an ad hoc local type.
- If a schema change would break partial rollout safety, codegen should force an
  explicit new branch into the generated reader union, and the writer-side type
  should follow that contract.
- CI should make that impossible to ignore by rejecting writes that are not
  accepted by the reader population you need to support.
- Prove what you can statically, then use fuzzing as a fallback when the checker
  cannot fully decide or when you want concrete examples.
- This does not mean every change becomes legal. Changes that introduce writer
  states unreadable by still-deployed readers are impermissible and should be
  blocked outright.
- One reason I like this workflow for agents too: current models often try to
  preserve backward compatibility in ugly, over-broad ways if you leave the
  boundary underspecified. It is useful to be able to say "never worry about
  backward compatibility except when the tests are yelling at you."
-->

---

<div class="deck-kicker">Final implication</div>

# One contract. Two local types.

<div class="deck-grid-3 mt-8 optional-soup-layout">
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
    <pre class="optional-soup-code"><code>type User = {
  name?: string | null
  city?: string | string[] | null
  eye_color?: string | null
  legacy_metadata?: unknown
}</code></pre>
  </div>
</div>

<div class="deck-callout optional-soup-callout mt-6">
  <p class="deck-quote">If old readers never go away, use a discriminated union of past types. Do not weaken one type until it means everything.</p>
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

<div class="deck-kicker">SRE playbook</div>

# Constrain. Split. Gate. Observe.

<div class="deck-grid-2 mt-8 sre-playbook-grid">
  <div class="law-card good">
    <h3>Constrain</h3>
    <p>Make strict schemas a cultural default: hidden assumptions should become contract rules, not tribal knowledge.</p>
  </div>
  <div class="law-card good">
    <h3>Split</h3>
    <p>Ship tooling that splits reader and writer types by default, and makes historical unions cheap to maintain.</p>
  </div>
  <div class="law-card good">
    <h3>Gate</h3>
    <p>Generate reader and writer contracts, check compatibility mechanically, and fail unsafe changes before merge.</p>
  </div>
  <div class="law-card good">
    <h3>Observe</h3>
    <p>Measure deserializations by payload version so you can see old tails, rollback risk, and when a branch is really gone.</p>
  </div>
</div>

<!--
End on durable company-level controls, not a one-off preflight for a single change.
-->

---
layout: center
---

<div class="thanks-slide">
  <div class="thanks-title">Thank you</div>
</div>
