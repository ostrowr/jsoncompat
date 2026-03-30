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
      <span class="hero-title-line">Escaping Version Skew</span>
    </div>
    <div class="hero-talk-subtitle">Formalizing compatibility in a world of partial rollouts</div>
    <div class="hero-talk-meta">Robbie Ostrow, Member of Technical Staff, OpenAI</div>
    <div class="hero-talk-event">SRECon Americas 2026</div>
  </div>
</NetworkHero>

<!--
- Structured data is constantly flowing between systems: edge, APIs, internal services, storage, and back again.
- In the steady state, this usually works well enough that it is easy to forget how much implicit coordination is involved.
- This talk is about defining better boundaries between systems so we can detect and prevent breaking changes automatically.
- The goal is strict contracts at abstraction boundaries without relying on humans to catch subtle compatibility bugs.
-->

---

<AudienceRolloutQuestion />

<!--
- A deploy starts and errors begin rising.
- The natural instinct is to halt the deploy and roll back.
- That is exactly what we did in this incident.
-->

---

# A mixed fleet shared one cache

<IncidentSketch />

<!--
- We had a load-bearing auth cache in Redis shared by a mixed fleet.
- Pods on the new version wrote a format that old pods could not parse.
- A request that hit a new pod and then later hit an old pod could fail on cache read.
- New pods could read both formats, but old pods could only read the old one.
- This caused up to a 15% error rate for ChatGPT for about 30 minutes, until the cache expired.
- A longer TTL would have made recovery much riskier.
-->

---
layout: center
---

<div class="incident-twist-slide">
  <h1>Rollback increased errors</h1>
  <p class="deck-quote mt-8">Old readers came back while bad cached data was still alive.</p>
</div>

<!--
- In this case, continuing the rollout would have been safer than rolling back.
- Once every pod was on the new version, all cache entries would have been readable again.
- We only understood that after the rollback had completed, so the safest recovery was to wait for expiry.
- This is not an argument against rollback; it is an example of how time makes systems harder to reason about.
- Humans, agents, and tests tend to reason about one version at one point in time, but real fleets are mixed-version systems.
- We need to reason about current, previous, and sometimes future versions at the same time.
- The systems we maintain most often break when they change, so we need a better theory of change.
-->

---
layout: center
---

<div class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</div>

<!--
- Breaking changes are often discussed as a client/server rollout ordering problem.
- The usual pattern is to expand what the server accepts, update the client, and then maybe re-constrain the server later.
- In practice, that last cleanup step is easy to forget, and dead compatibility code accumulates.
- This approach also depends on humans rolling services out in the right order.
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-word">give up</div>
</div>

<!--
- Give up on manually coordinating ordered rollouts as a correctness strategy.
- Do not give up on correctness itself.
- If a change in trunk only works when services deploy in a specific order, it is too fragile.
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase emphasis-phrase-coral">don't rely on humans</div>
</div>

<!--
- Do not rely on humans or agents to catch breaking changes in an unconstrained system.
- Manual rollout ordering is hard to reason about, makes rollbacks unsafe, and can be impossible when dependencies are circular.
- We need better constraints at abstraction boundaries and better tooling to enforce them.
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
- Any durable or in-flight state adds a time dimension to schema changes: caches, queues, databases, and RPCs all matter.
- A static architecture diagram hides the fact that old and new versions overlap during rollout.
- In this example, we add `eye_color` and want to make it required for all future users.
- If the reader rolls out first, it starts rejecting payloads from the old writer.
- Even if readers and writers flipped at exactly the same time, in-flight requests could still fail.
- Errors only stop after both sides have been on the same version long enough for queues and RPCs to drain.
-->

---

# Parseable is not enough

<div class="one-figure-slide pydantic-compat-example mt-8">
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
- Protobufs solve a lot of wire compatibility problems, but they do so by weakening the set of states the schema can represent.
- Optional-heavy schemas make compatibility easier by pushing constraints out of the contract and into application code.
- If an invariant matters to business logic, it should be enforced at the boundary.
- Application code should not have to handle every historical state forever.
- Schema contracts should encode richer rules than parseability alone, whether through JSON Schema or validation layers like Protovalidate.
- Stricter schemas make breaking changes easier to introduce, so we need tooling to manage that safely.
-->

---

# Avoid optionalslop

<div class="deck-grid-2 optional-soup-layout mt-8">
  <div class="deck-schema-box optionalslop-grotesque">

```proto
message UserProfile {
  optional string display_name = 1;
  optional string first_name = 2;
  optional string last_name = 3;
  optional string legacy_full_name = 4;
  optional string avatar_url = 5;
  optional string avatar_id = 6;
  optional string locale = 7;
  optional string timezone = 8;
  optional bool email_verified = 9;
  optional bool phone_verified = 10;
  optional string phone_number = 11;
  optional string backup_phone_number = 12;
  optional string city = 13;
  optional string region = 14;
  optional string country = 15;
  optional string legacy_metadata_json = 16;
}
```

  </div>
  <div class="fact-card boundary-card optionalslop-copy">
    <div class="optionalslop-stamp">compatibility residue</div>
    <div class="boundary-point">
      <div class="boundary-point-title">One type gets weaker over time</div>
      <div class="boundary-point-body">As old fields accumulate for compatibility, the shared proto stops expressing the real domain model and turns into "maybe this, maybe that".</div>
    </div>
    <div class="boundary-point">
      <div class="boundary-point-title">Impossible states become routine</div>
      <div class="boundary-point-body">Now business logic has to remember which subsets belong together, which are stale, and which combinations should never exist.</div>
    </div>
  </div>
</div>

<!--
- Current tooling encourages what I call optionalslop.
- Every migration, rollback path, and compatibility tail leaves residue in the schema.
- The result is a type that stays wire-compatible but gets worse at expressing which states are valid today.
- That pushes cleanup and correctness into business logic everywhere the type is read.
- If the contract allows impossible combinations, a future developer will eventually send one.
- This is not an argument against protobufs; it is an argument for pairing a wire format with stronger boundary rules.
- The boundary should be written in a schema definition language that can generate code without sneaking in extra assumptions.
-->

---

# Strict contracts are better for ~~humans~~ agents

<div class="deck-grid-3 mt-8 agent-contract-grid">
  <div v-click class="law-card success">
    <h3>Smaller legal state space</h3>
    <p>Fewer ambiguous shapes for an agent depend on.</p>
  </div>
  <div v-click class="law-card success">
    <h3>Hidden assumptions become explicit</h3>
    <p>Put the rule at the boundary so the agent does not have to recover it.</p>
  </div>
  <div v-click class="law-card success">
    <h3>Crisper test oracle</h3>
    <p>A strict contract allows an agent loop to quickly iterate upon correctness.</p>
  </div>
</div>

<div v-click class="deck-callout mt-8">
  <p class="deck-quote">Agentic workflows get safer when the boundary is narrow enough to make bad states impossible, not just unlikely.</p>
</div>

<!--
- Stronger models do not remove the need for strict contracts; they make strict contracts more important.
- Agents, like humans, can build systems too large to fully understand at once.
- As models get stronger, abstraction boundaries can get larger, but the systems behind them will also get larger.
- We still need contracts that fit inside the reasoning budget of the agent or human changing the system.
- Put as many constraints into the contract as possible, especially when business logic depends on them.
- Strict contracts reduce the legal state space, make hidden assumptions explicit, and create a sharper oracle for CI, review, and agentic loops.
- Do not make developers or agents re-derive implicit contracts from surrounding code and tribal knowledge.
- A mechanical guarantee is much better than a runbook for minimizing cognitive overhead.
-->

---

# Stop sharing types.

<div class="deck-grid-2 mt-10 writer-reader-principle subsumption-containment-grid">
  <div class="law-card success">
    <h3>Writers should be as strict as possible</h3>
    <p>Emit today's contract, not a mushy superset shaped by every historical rollout.</p>
  </div>
  <div class="law-card success">
    <h3>Readers should accept the union of the last few writers</h3>
    <p>Carry compatibility in the reader, where skew actually lands.</p>
  </div>
</div>

<div class="deck-callout mt-10">
  <p class="deck-quote">Stop sharing types between client and server.</p>
</div>

<!--
- The practical version of strict contracts starts with changing how we think about shared types.
- Stop sharing one type between client and server, or between serializer and deserializer.
- Replace shared types with a schema DSL that can generate separate writer and reader types.
- Writers should be as strict as possible and emit today's contract only.
- Readers should carry the compatibility burden by accepting the union of the last few writer versions.
- Shared types feel simpler, so separate reader and writer types only work if the tooling makes them the easy path.
-->

---

# A strict writer, a union reader

<div class="deck-grid-2 mt-8">
  <div class="one-figure-slide pydantic-compat-example">

```python
class UserProfileWriter(BaseModel):
    name: str = Field(min_length=1)
    age: int = Field(ge=0)
```

  </div>
  <div class="one-figure-slide pydantic-compat-example">

```python
type UserProfileReader =
    | UserProfileV1Reader
    | UserProfileV2Reader
    | UserProfileV3Reader

match payload:
    case UserProfileV3Reader(name=name, age=age):
        ...
    case UserProfileV2Reader(full_name=full_name):
        ...
```

  </div>
</div>

<div class="deck-callout mt-8">
  <p class="deck-quote">New writes stay clean. Compatibility is quarantined to explicit old-version branches.</p>
</div>

<!--
- The writer type should represent today's truth only.
- If all new users have a name and an age, the writer should enforce exactly that.
- The reader is where we pay the compatibility cost, as an explicit union of old writer shapes that still need to be accepted.
- Code branches on the versioned shape instead of hiding compatibility guesses in one giant optional type.
- This quarantines historical behavior so new writes stay clean and old branches are obvious, local, and eventually deletable.
- No one wants to maintain that reader type by hand, so it needs to be generated.
-->

---

# Stamp every payload with a writer version.

<div class="deck-grid-2 stamp-process-intro mt-6">
  <div class="law-card success">
    <h3>Writers stamp the shape they emitted</h3>
  </div>
  <div class="law-card success">
    <h3>Readers branch on the stamp, not on custom logic</h3>
  </div>
</div>

<div class="tooling-checklist tooling-checklist-compact stamp-process-checklist mt-6">
  <div v-click class="tooling-step"><strong>1</strong><span>Update the schema.</span></div>
  <div v-click class="tooling-step"><strong>2</strong><span>Detect breaking changes.</span></div>
  <div v-click class="tooling-step"><strong>3</strong><span>Keep the writer as strict as possible.</span></div>
  <div v-click class="tooling-step"><strong>4</strong><span>Make readers a tagged union of the last few writers.</span></div>
  <div v-click class="tooling-step"><strong>5</strong><span>Measure how often old writer branches still deserialize.</span></div>
  <div v-click class="tooling-step"><strong>6</strong><span>Delete old branches once those metrics hit zero.</span></div>
</div>

<!--
- The source of truth should be a contract in a schema DSL: JSON Schema, proto plus Protovalidate, or something equivalent.
- On every schema change, use static analysis where possible and fuzzing otherwise to check whether the change is breaking under partial rollout, and in which direction.
- Compatibility is a property of the contract, not of whatever data happens to be flowing today.
- If a change is breaking in either direction, CI should require a new stamped type.
- Writers use only the newest strict type, while readers use a tagged union of the historical writer types they still need to accept.
- Generated code should ideally make it impossible to serialize from reader types or deserialize from writer types.
- This moves cross-version reasoning out of engineers' heads and into CI.
- Schema versions become explicit points in time, which makes old branches measurable and eventually deletable.
- The full workflow requires breaking-change detection, telemetry, code generation, historical schema lookup in CI, and an escape hatch for brand-new evolving schemas.
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
- For non-breaking changes, CI should allow the change with no extra workflow.
- For a breaking change like changing a field type, CI should force a staged rollout.
- In this example, `interests` changes from a list of strings to an integer.
- CI first rejects updating the writer because the reader on main does not yet accept the new version.
- After stamping, readers accept both the old and new versions.
- Once metrics show all readers are rolled out, CI allows the new strict writer type to merge.
- Only after the old data tail is gone do we remove the old reader branch.
- Writer/reader terminology is clearer than forward/backward compatibility because many systems both deserialize input and serialize output.
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase">Tooling!</div>
  <div class="hero-talk-subtitle mt-4">Prove when possible. Fuzz when not.</div>
</div>

<!--
- The hard part is detecting breaking changes in an expressive contract language.
- Protobuf compatibility rules are relatively simple.
- JSON Schema compatibility is much harder because the language is much more expressive.
- That expressiveness is useful, but it makes compatibility analysis substantially more difficult.
-->

---
class: demo-full-bleed
---

<CheckerEmbed />

<!--
- `jsoncompat` is a generic JSON Schema subsumption checker.
- It statically analyzes schema changes to detect whether they are breaking.
-->

---

# A subsumption checker asks set containment

<div class="deck-grid-2 mt-4 writer-reader-principle">
  <div class="law-card success">
    <h3>New writer safe for old reader</h3>
    <p>L(new) ⊆ L(old)</p>
  </div>
  <div class="law-card success">
    <h3>Old writer safe for new reader</h3>
    <p>L(old) ⊆ L(new)</p>
  </div>
</div>

<div class="deck-callout mt-2">
  <p class="deck-quote">A schema change is compatible in a direction exactly when every value accepted before is still accepted after, or vice versa.</p>
</div>

<div class="assumption-footnote mt-3">
  Serializer assumption: no extra emitted fields beyond the declared schema.
</div>

<!--
- A schema denotes a language of valid JSON values: `L(schema)`.
- A subsumption checker asks whether one schema's language is a subset of another's.
- If `L(new) ⊆ L(old)`, a new writer is safe for an old reader.
- If `L(old) ⊆ L(new)`, an old writer is safe for a new reader.
- When either relation fails, the checker should produce a witness value if possible.
- This is hard because the language usually cannot be enumerated directly.
- `jsoncompat` needs keyword-specific logic to prove containment across recursive schemas, sum types, regexes, and other constraints.
- The checker can only reason about invariants encoded in the schema itself.
-->

---

# Two passes: prove, then search

<div class="deck-grid-2 mt-10 writer-reader-principle subsumption-containment-grid">
  <div class="law-card success">
    <h3>Static checker</h3>
    <p>Fast, deterministic proofs for the common cases.</p>
  </div>
  <div class="law-card success">
    <h3>Fuzzer</h3>
    <p>Concrete counterexamples when the schema is too expressive for a complete proof.</p>
  </div>
</div>

<div class="tooling-checklist tooling-checklist-compact mt-8">
  <div class="tooling-step"><strong>1</strong><span>Try to prove set containment from the schemas alone.</span></div>
  <div class="tooling-step"><strong>2</strong><span>If the proof is incomplete, search for a witness value.</span></div>
  <div class="tooling-step"><strong>3</strong><span>Use the witness to make the breakage obvious to humans and agents.</span></div>
</div>

<!--
- The workflow has two passes: prove first, then search.
- For many ordinary schema changes, compatibility can be proven directly from the schemas.
- Static checks are fast, deterministic, and easy to run in CI.
- When a complete proof is not practical, fuzzing can search for a concrete witness value accepted on one side and rejected on the other.
- Proofs are ideal when available; examples are the fallback when the schema is too expressive.
- Witnesses are useful for CI, code review, and incident debugging.
-->

---

# A concrete witness makes breakage obvious

<div class="witness-slide-shell mt-5">
  <div class="witness-schema-panel witness-schema-old">
    <div class="witness-label">Old schema</div>

```json {all|10}
"if": { "properties": { "mode": { "const": "percent" } } },
"then": {
  "properties": {
    "value": { "maximum": 100 }
  }
}
```

  </div>
  <div class="witness-change-rail">
    <div class="witness-arrow">→</div>
    <div class="witness-change-copy">one keyword tightens</div>
  </div>
  <div class="witness-schema-panel witness-schema-new">
    <div class="witness-label">New schema</div>

```json {all|4}
"if": { "properties": { "mode": { "const": "percent" } } },
"then": {
  "properties": {
    "value": { "exclusiveMaximum": 100 }
  }
}
```

  </div>
</div>

<div class="witness-result mt-6">
  <div class="witness-result-kicker">Witness</div>
  <code>{"mode":"percent","value":100}</code>
  <div class="witness-result-copy">Valid before. Rejected after. </div>
</div>

<!--
- A concrete witness makes compatibility failures much easier to understand.
- This example tightens a conditional schema from `maximum: 100` to `exclusiveMaximum: 100`.
- The witness is `{"mode":"percent","value":100}`.
- That payload was valid before and rejected after.
- In a large schema, one concrete payload is often more useful than abstract compatibility prose.
-->

---
class: demo-full-bleed
---

<CheckerEmbed />

<!--
- The checker on `jsoncompat.com` is written in Rust and compiled to WebAssembly for browser use.
- A simple object schema with `name` and `age` can be checked against itself to confirm compatibility.
- Tightening `minLength` from the old schema to the new schema is a breaking change for readers, because old data can still contain shorter names.
- The static checker works for most schemas, so fuzzing is usually not needed.
- Some JSON Schema features, such as `not`, conditionals, cross-field constraints, and complex composition, are much harder to decide statically.
- In those cases, the workflow falls back to fuzzing for concrete counterexamples.
- The fuzzer is also useful as a test harness for improving the static checker itself.
- `jsoncompat` is MIT-licensed and has already been useful for catching breaking changes at storage boundaries.
-->

---

# Make compatibility checks live next to the type

<div class="one-figure-slide pydantic-compat-example mt-8">

```python
from pydantic import BaseModel, Field

@jsoncompat_check(direction="both", stable_id="user-profile")
class UserProfile(BaseModel):
    name: str = Field(min_length=1)
    age: int = Field(ge=0)
```

</div>

<div class="deck-callout mt-8">
  <p class="deck-quote">The stable ID ties this model to its historical schema snapshots, and CI checks both rollout directions on every change.</p>
</div>

<!--
- You do not need to adopt schema-first code generation all at once to get value from compatibility tooling.
- At OpenAI, many Pydantic models define data stored in databases, Redis, and other durable systems.
- Those models can be decorated with a `jsoncompat` check that snapshots the current schema and compares it against previous commits in CI.
- This is less powerful than separate generated reader and writer types, but it still catches subtle breaking changes.
- The compatibility policy lives next to the type definition, and the stable ID preserves identity across renames and refactors.
- With `direction="both"`, changes must be safe for old readers seeing new writes and for new readers seeing old writes.
- The auth cache type from the incident is now protected by this checker.
- Tests can enforce that important storage-boundary models always have compatibility checks.
-->

---

# Adopt it in phases

<div class="tooling-checklist">
  <div class="tooling-step"><strong>1</strong><span>Start by annotating storage-boundary types and checking both rollout directions in CI.</span></div>
  <div class="tooling-step"><strong>2</strong><span>Add writer-version stamps and measure which old branches are still being read.</span></div>
  <div class="tooling-step"><strong>3</strong><span>Split strict writer types from union reader types on the boundaries that matter most.</span></div>
</div>

<div class="deck-callout mt-4">
  <p class="deck-quote">You do not need the whole end-state on day one to start catching real breakages.</p>
</div>

<!--
- Adoption can happen in phases.
- Start by annotating types on real storage or queue boundaries and checking both rollout directions in CI.
- Next, stamp writer versions and measure the old read tail so rollback risk and cleanup timing are visible.
- Then split strict writer types from union reader types on the boundaries that matter most.
- You do not need the full end-state on day one to start moving risky boundaries from vibes to mechanical checks.
-->

---

# When not to do this

<div class="deck-grid-2 mt-8">
  <div class="law-card">
    <h3>Probably not worth it</h3>
    <p>Ephemeral internal RPCs with no durable state, no queues, and no meaningful rollback tail.</p>
  </div>
  <div class="law-card success">
    <h3>Absolutely worth it</h3>
    <p>Caches, queues, databases, durable workflows, mobile or external clients, and any boundary where state outlives binary.</p>
  </div>
</div>

<div class="deck-callout mt-8">
  <p class="deck-quote">Use the heavy machinery where old code and new state can meet. That is where version skew turns into incidents.</p>
</div>

<!--
- Not every boundary needs this much machinery.
- Ephemeral internal RPCs with no durable state, queues, or rollback tail may be fine with ordinary API compatibility discipline.
- This is absolutely worth it for caches, queues, databases, durable workflows, mobile clients, external clients, and any boundary where state outlives a binary.
- The filter is simple: use the machinery where old code and new state can meet.
- That is where version skew turns into incidents.
-->

---

# Constrain. Split. Gate. Observe.

<div class="deck-grid-2 mt-8 sre-playbook-grid">
  <div class="law-card good">
    <h3>Constrain</h3>
    <p>Make strict schemas a cultural default: hidden assumptions should become contract rules, not tribal knowledge.</p>
  </div>
  <div class="law-card good">
    <h3>Split</h3>
    <p>Generate reader and writer types in your language of choice from the schema, and make historical unions cheap to maintain.</p>
  </div>
  <div class="law-card good">
    <h3>Gate</h3>
    <p>Run CI against the schema itself and against previous versions, detect breakages mechanically, and fail unsafe changes before merge.</p>
  </div>
  <div class="law-card good">
    <h3>Observe</h3>
    <p>Measure deserializations by payload version so you can see old tails, rollback risk, and when a branch is really gone.</p>
  </div>
</div>

<!--
- Constrain: make strict schemas the default so hidden assumptions become contract rules instead of tribal knowledge.
- Split: generate separate reader and writer types from the schema, and make historical reader unions cheap to maintain.
- Gate: run CI against the schema and its previous versions to detect breaking changes before merge.
- Observe: measure which old versions are still being read so rollback risk and cleanup timing are visible.
- The goal is not a one-off preflight for scary changes, but a system that makes the safe path the normal path.
-->

---
layout: center
---

<div class="thanks-slide">
  <div class="thanks-title">Questions?</div>
  <a class="thanks-link" href="https://jsoncompat.com">slides and tooling at jsoncompat.com</a>
</div>

<!--
- Slides and tooling are available at `jsoncompat.com`.
- Questions are welcome.
-->
