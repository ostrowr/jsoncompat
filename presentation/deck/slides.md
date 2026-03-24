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
Structured data is flowing between your systems. Some request hits the edge, which hits some API service, which maybe makes a couple more hops within your infrastructure. In the steady state – if you have enough capacity – it just works. 

I'm Robbie Ostrow - I work on infra at OpenAI. Our systems are big, and constantly growing and evolving, often in ways that are too complex for me, or more importantly, gpt 5.4, to understand. The short version of this talk is the title: escaping version skew. The subtitle is the method. I want to talk about how we can define better boundaries between systems, detect and prevent breaking changes, maintain strict contracts at those boundaries, and do it automatically instead of relying on humans to catch subtle incompatibilities.
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
So, in this particular case, we would have been better served by letting the rollout continue. Once all of the pods were on the new version, everyone could have read all the cache entries, and we wouldn't have seen the secondary error spike. Sadly, we didn't realize this until the rollback completed, and by that time the safest way to recovery was to let the bad cache entries expire on their own.

Now, this is not to say you shouldn't roll back. Rollback first, ask questions later is a good motto. But it just shows that as soon as you add the dimension of time into your systems, they get so so much more complicated to understand. Humans, agents, and tests tend to look at a single point of time, a single hash. That's convenient for understanding your system, but it's a lie once your systems get above toy-sized. We have to think about the infrastructure we run not at a single point of time, but also at all of the previous versions (and in some case future versions) that are running across our fleets.
-->

---
layout: center
---

<div class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</div>

<!--
When we talk about breaking changes, we're usually talking about clients and servers. You've probably babysat many annoying changes of the flavor "let's expand what the server accepts first, then update the client to send the new thing, then hopefully remember to re-constrain the server to only accept the new thing. Usually we forget to do the third step. And you have to make sure that you remember to roll out the services in the right order. So, I'm going to begin this talk by telling you the secret to coordinating ordered rollouts in a system the size of OpenAI's:
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
  <div class="emphasis-phrase emphasis-phrase-coral">Don't rely on rollout order.</div>
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
Let's zoom in a bit. Any time you have state, whether that's in a shared cache, a queue, a database, or even just an inflight RPC between services, you lose the luxury of imagining your system as the nice, static diagram you write in your docs. Instead, you have to add that additional time dimension to your thinking.

Let's imagine I want to add a new field to this schema, eye-color. 

(s)

Oops, it rolled out to the reader first! eye_color is required, so we're seeing some errors. It's ok, let's roll out the writer too. 

(s, p)

Look at this - even if all the readers and all the writers had flipped at exactly the same time, inflight requests would have failed. 

It's only now that the reader and the writer have been at the same version for a while (all the queues and RPC have drained) will we stop seeing errors.
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
OK, you're thinking. We solved this problem in like 2001 with protobufs. Why is robbie up there complaining about a solved problem? surely someone has told him about protos. 

Well, yes. Protos do indeed solve the wire compatibility problem. But they do so by substantially weakening the set of states they can represent. Protos, what with their optional-only fields and no logic constraints -  make wire compatibility easy - but you're sacrificing constraints in your application's for ease of wire compat. If you want your systems to have these solid abstraction boundaries - which I very much do - your invariants belong at the boundaries of your systems. Your application code should not be in the business of dealing with old versions of stuff forever, leaving dead branches of code that you can never prune, and generally leaving your application developers in a situation where they have to handle all possible sets of states from all time. 

So when I talk about defining schemas, I'm not just talking about the wire format. I'm also talking about contracts that can encode powerful rules, like json schema or extensions to protos like protovalidate.

Unfortunately, though, the stricter your schemas are, the easier it is for you to make a so-called "breaking change."
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
This is the maintenance smell I want to name: long-lived proto evolution often
turns one shared type into a pile of optionals. Every migration, rollback path,
and compatibility tail leaves residue in the schema. I want this slide to feel
a little grotesque, because that's what it is: a type that is technically
compatible but increasingly bad at expressing which states are actually valid
now. This type doesn't make impossible states unrepresentable. It normalizes
impossible states, and pushes the cleanup burden into business logic everywhere
that reads it.

I promised at the beginning that we can keep things safe while also reducing cognitive overhead and making impossible states impossible. We do this with tooling.
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
      <div class="boundary-point-body">If the rule is <code>&lt; 5</code>, encode <code>&lt; 5</code> in the contract!</div>
    </div>
    <div class="boundary-point">
      <div class="boundary-point-title">Guarantee</div>
      <div class="boundary-point-body">Only schema invariants are guaranteed. Reject bad input at the boundary.</div>
    </div>
  </div>
</div>

<!--
I don't want to give you the feeling that I'm anti-proto. Certainly not. It's just not enough. We need a wire format PLUS additional rules, because at the end of the day, only the abstraction boundary is guaranteed. And you'd better be writing that boundary in some schema definition language so you can generate code from it. 

We're going to use JSON schema as the contract definition language for the rest of this talk. The same ideas apply to nearly any powerful schema definition language, but JSON is pretty ubiquitous at OpenAI and elsewhere.

I just want to hammer this point home. Put as many constraints into your contract as possible. Retries isn't just an integer, it's an integer between 0 and 4. Mode isn't a string, it's either fast or safe. All of the fields are required. Then, you can generate types for handlers that fulfill this contract, and you don't have to worry about handling the case where mode is missing or malformed. We check that at the edge, so our business logic can be simple and correct.
-->

---

# Writers strict. Readers wide.

<div class="deck-grid-2 mt-10 writer-reader-principle">
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
So, this is the shape I actually want. Writers should be as strict as possible.
They should emit today's contract, not some giant compromise type that has been
weakened by every migration you've ever done.

Readers, on the other hand, are where the compatibility burden belongs. They
should accept the union of the last few writer versions, because that's where
skew lands in practice.

And I know everyone wants to share types between client and server. Of course
they do. It feels simpler, cheaper, cleaner. So if you want people not to do
that, the tooling has to be really good. Separate writer and reader types can't
feel like a tax. They have to feel like the easy path.
-->

---

# Stamp every payload with a writer version.

<div class="deck-grid-2 mt-10 writer-reader-principle">
  <div class="law-card success">
    <h3>Writers stamp the shape they emitted</h3>
    <p>Add an explicit payload version or schema ID at the boundary, so a reader knows which historical branch it is parsing.</p>
  </div>
  <div class="law-card success">
    <h3>Readers branch on the stamp, not on vibes</h3>
    <p>Make compatibility explicit and observable: parse by version, count by version, and delete by version when the tail is gone.</p>
  </div>
</div>

<!--
So what does that mean mechanically?

First, stamp the payloads. If the reader is going to carry a small historical
union, it needs to know which branch it is looking at. So writers should emit
an explicit payload version or schema ID at the boundary. Then readers can
branch on that stamp instead of guessing from shape, and you can measure
exactly which old versions are still showing up in production.
-->

---

# Today's contract for writers. A small union for readers.

<div class="tooling-checklist tooling-checklist-compact mt-5">
  <div class="tooling-step"><strong>1</strong><span>Update the schema.</span></div>
  <div class="tooling-step"><strong>2</strong><span>Detect breaking changes.</span></div>
  <div class="tooling-step"><strong>3</strong><span>Keep the writer as strict as possible.</span></div>
  <div class="tooling-step"><strong>4</strong><span>Make readers a tagged union of the last few writers.</span></div>
  <div class="tooling-step"><strong>5</strong><span>Measure how often old writer branches still deserialize.</span></div>
  <div class="tooling-step"><strong>6</strong><span>Delete old branches once those metrics hit zero.</span></div>
</div>

<!--
Then the six-step process is straightforward.

You update the schema. The tooling checks whether that change is breaking under
partial rollout. If it is, you don't make the writer sloppy. You keep the
writer strict, and you widen the reader into a tagged union of the last few
writer shapes you still need to handle.

And then you measure it. How often are we still deserializing the old
branches? When that drops to zero, you don't have to guess. You have a real
signal that it's time to delete the old branch.

That's the workflow I want: strict current writers, explicit historical readers,
and a cleanup loop that is driven by production evidence instead of vibes.
-->

---

# Strict contracts are better for ~~humans~~ agents

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
This is good for humans, obviously. But I think it's even better for agents.

Models are not great at reconstructing your implicit invariants from a pile of
surrounding code and tribal knowledge. If the boundary is loose, they will
invent plausible-looking states that are subtly wrong.

If the contract is strict, the legal state space is smaller. Hidden assumptions
become explicit. And you get a much sharper oracle for CI and review than
"looks reasonable to me."

So the pitch here is not just "this is cleaner architecture." It's that strict
contracts make agentic workflows safer, because they make more bad states
impossible instead of merely unlikely.
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
This is the constructive version of the rollout.

First you deploy the reader union, so the new code can still parse the old
shape and the new shape. Then you deploy the writer change and start emitting
the new version. And only after the old data tail is gone do you remove the old
branch.

That's the answer to "okay, if I don't rely on rollout order and I don't want
to let things break, what do I do instead?" You make the mixed-version period a
first-class thing in the types.
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase">Check it mechanically.</div>
</div>

<!--
And because humans are bad at reasoning about this in their heads, we should
check it mechanically.
-->

---
class: demo-full-bleed
---

<CheckerEmbed />

<!--
For the demo, I want to show the nice case first: some breaking changes are
detectable statically.

I'll take the schema with `exclusiveMaximum: 5` and tighten it to `4`. The
checker can prove that's unsafe directly from the old and new contracts. It
doesn't need to hunt for examples. It can tell us the witness is `4`: old
writers can still emit it, and new readers will reject it. That's exactly the
kind of thing you want CI to catch before merge.

But for a sufficiently powerful constraint language, not every compatibility
question is that easy. Once you have richer combinations of conditionals,
cross-field constraints, and schema composition, there are changes where a
complete static answer is much harder or not practical.

So then we need fuzzing too. The workflow is: prove what we can statically, and
when the checker can't fully decide, search for concrete counterexamples. I
like that combination a lot, because the static checker gives you fast,
deterministic guardrails, and fuzzing gives you real examples when the logic
gets too expressive for a complete proof.
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
So if I were going to summarize this as an SRE-ish playbook, it would be these
four things.

Constrain: make strict schemas the default, so hidden assumptions become
contract rules instead of tribal knowledge.

Split: generate separate reader and writer types by default, and make the
historical reader union cheap to maintain.

Gate: check compatibility mechanically in CI, and fail unsafe changes before
they merge.

Observe: measure which old versions are still being read, so you can see the
tail, understand rollback risk, and know when cleanup is actually safe.

That's the durable version of this. Not a one-off preflight for one scary
change, but a system that makes the safe path the normal path.
-->

---
layout: center
---

<div class="thanks-slide">
  <div class="thanks-title">Questions?</div>
  <a class="thanks-link" href="https://jsoncompat.com">slides and tooling at jsoncompat.com</a>
</div>
