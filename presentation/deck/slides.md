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
Even if there's a ton of complexity behind any one of these nodes, you can rely on your trusty architecture diagram and the interfaces between your systems.

I'm Robbie Ostrow - I work on infra at OpenAI. Our systems are big, and constantly growing and evolving, often in ways that even I, or more importantly, gpt 5.4 can't understand. I want to talk to you today a bit about how we can define better boundaries between our systems and detect when you're about to ship a change without breaking your systems as it rolls out.
-->

---

<AudienceRolloutQuestion />

<!--
First, a question for you all - and don't worry, I promise this is the only interactive portion of the talk. The date is early 2025. We're seeing errors rise with a deploy. What do you do? Just yell it out.

<revert>

Well, halt the deploy and roll back is exactly what we did. Let me show you what happened.
-->

---

# A mixed fleet shared one cache

<IncidentSketch />

<!--
See, we had a load bearing auth cache in redis. Pods running the new version were writing a type that the old version couldn't understand - so anyone who hit a new pod to fill out the auth cache then later hit an old pod on a subsequent request would get an error parsing the data from the cache. 

The new pods could read the new format and the old format, but the old pods could only read the old format. This ended up causing up to a 15% error rate for chatgpt for about 30 minutes, until everything in the cache expired. We were lucky that the TTL wasn't very long.
-->

---
layout: center
---

<div class="incident-twist-slide">
  <h1>Rollback increased errors</h1>
  <p class="deck-quote mt-8">Old readers came back while bad cached data was still alive.</p>
</div>

<!--
So, in this particular case, we would have been better served by letting the rollout continue. Once all of the pods were on the new version, everyone could have read all the cache entries, and we wouldn't have seen the secondary error spike. Sadly, we didn't realize this until the rollout completed, and by that time the safest way to recovery was to let the bad cache entries expire on their own.

Now, this is not to say you shouldn't roll back. Rollback first, ask questions later is a good motto. But it just shows that as soon as you add the dimension of time into your systems, they get so so much more complicated to understand. Humans, agents, and tests tend to look at a single point of time, a single hash. That's convenient, but it's a lie once your systems get above toy-sized, we have to think about them  not just as the set of things running on the current version, but also at all of the previous versions (and in some case future versions) that are running across our fleets. [TODO, add title slide zooming in]
-->

---
layout: center
---

<div class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</div>

<!--
When we talk about breaking changes, we're usually talking about clients and servers. You've probably submitted many annoying changes of the flavor "let's expand what the server accepts first, then update the client to send the new thing, then hopefully remember to constrain the client to only accept the new thing. Usually we forget to do the third step. And you have to make sure that you remember to roll out the services in the right order. So, I'm going to begin this talk by telling you the secret to coordinating ordered rollouts in a system the size of OpenAI's:
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-word">give up</div>
</div>

<!--
You should give up! Don't give up on correctness, mind you – but if you have a change in trunk that will only work if services are deployed in a certain order, you're going to have a bad time.
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase">Don't rely on rollout order.</div>
</div>

<!--
You shouldn't rely on rollout order. It's too complicated to reason about, it makes rollbacks unsafe, there are sometimes circular dependencies that make it entirely impossible. We need a better solution than a human manually gating rollouts to make sure their breaking change gets into production successfully.
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
Any time you have state, whether that's in a shared cache, a queue, a database, or even just an inflight RPC between services, you stop getting the luxury of imagining your system as the nice, static diagram you write in your docs. Instead, you have to add that additional time dimension to your thinking.

Let's imagine I want to add a new field to this schema, eye-color. 

(s)

Oops, it rolled out to the reader first! eye_color is required, so we're seeing some errors. It's ok, let's roll out the writer too. 

(s, p)

Look at this - even if all the readers and all the writers had flipped at exactly the same time, inflight requests would have failed. 

Now we're back in that happy, static world. Until the next deploy.
-->

---

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
OK, you're thinking. We solved this problem in the 2001 with protobufs. Why is robbie up there complaining about a solved problem? surely someone has told him about protos. 

Well, yes. Protos do indeed solve the wire compatibility problem. But they do so by substantially weakening the set of states they can represent. There's no longer such thing as a required field in protos. That makes wire compatibility easy - but you're sacrificing application-level constraints for ease of wire compat. If you want your systems to have solid abstraction boundaries - which I very much do - your invariants belong at the boundaries of your systems. Your application code should not be in the business of dealing with old versions of stuff forever, leaving dead branches of code that you can never prune, and generally leaving your application developers in a situation where they have to handle all possible sets of states from all time. 

So when I talk about defining schemas, I'm talking about both grammar-based schemas like protos, but also schemas that can encode powerful rules, like json schema or extensions to protos like protovalidate.

Ultimately, if your business logic depends on some shape of data, you should reject bad data at the boundary at the edge. But unfortunately, the stricter your schemas are, the easier it is for you to make a so-called "breaking change."
-->

---

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
[TODO maybe remove this slide?] 

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
Only the contract is guaranteed. Put as much into the contract as possible, but ensure that your business logic does not make any assumptions that are not encoded into the contract that you're publishing for your clients.
-->

---

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
      <div class="boundary-point-body">If the rule is <code>&lt; 5</code>, write <code>&lt; 5</code>.</div>
    </div>
    <div class="boundary-point">
      <div class="boundary-point-title">Guarantee</div>
      <div class="boundary-point-body">Only schema invariants are guaranteed. Reject bad input at the boundary.</div>
    </div>
  </div>
</div>

<!--
So, we're going to use JSON schema as the contract definition language in the rest of this talk. The same ideas apply to nearly any powerful schema definition language, but JSON is pretty ubiquitous at OpenAI and elsewhere. 

I just want to hammer this point home. Put as many constraints into your contract as possible. Retries isn't just an integer, it's an integer between 0 and 4. Mode isn't a string, it's either fast or safe. All of the fields are required. Then, you can generate types for handlers that fulfill this contract, and you don't have to worry about handling the case where mode is missing or malformed. We check that at the edge.
-->

---

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
This contract point is important for humans and for agents. I feel 

Make the agent point concrete and engineering-focused:
- Large model callers are especially bad at reconstructing implicit invariants
  from surrounding context.
- Tight cotracts reduce the amount of hidden reasoning the agent has to do.
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
[TODO] move this after the contract point, and add a diagram

JSON schema -> writer type -> union of last few deserializer types


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
