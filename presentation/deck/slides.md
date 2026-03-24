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

I'm Robbie Ostrow - I work on infra at OpenAI. Our systems are big, and constantly growing and evolving, often in ways that are too complex for me, or more importantly, gpt 5.4, to understand. This talk is called Escaping Version Skew. I want to talk to you today a bit about how we can better define boundaries between our systems. We should be able to detect and prevent breaking changes. We should be able to maintain strict contracts at these abstraction boundaries. And we should be able to do all of this automatically, without relying on humans to catch subtle breaking changes.

[48s]
-->

---

<AudienceRolloutQuestion />

<!--
First, a question for you all - and don't worry, I promise this is the only interactive portion of the talk. The date is early 2025. We're seeing errors rise with a deploy. What do you do? Just yell it out.

<revert>

Well, halt the deploy and roll back is exactly what we did. Let me show you what happened.

[20s]
-->

---

# A mixed fleet shared one cache

<IncidentSketch />

<!--
See, we had a load bearing auth cache in redis. Pods running the new version were writing a type that the old version couldn't understand - so anyone who hit a new pod to fill out the auth cache then later hit an old pod on a subsequent request would get an error parsing the data from the cache. 

The new pods could read the new format and the old format, but the old pods could only read the old format. This ended up causing up to a 15% error rate for chatgpt for about 30 minutes, until everything in the cache expired. We were lucky that the TTL wasn't very long, because otherwise we might have had to do a risky manual production operation.

[35s]
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

Now, this is not to say you shouldn't roll back. Rollback first, ask questions later is a good motto. But it just shows that as soon as you add the dimension of time into your systems, they get so so much more complicated to understand. Hell, I was confusing myself on the previous slide talking about new versions talking to old versions talking to new versions, and that was only one service talking to one storage layer. It only gets more complicated than that. Humans, agents, and tests tend to look at a single point of time, a single hash. That's convenient for understanding your system, but it's a lie once your systems get above toy-sized. We have to think about the infrastructure we run not at a single point of time, but also at all of the previous versions (and in some case future versions) that are running across our fleet and potentially customer fleets or clients. Our systems tend to break when they change, and we need a better theory of change.

[1:15]
-->

---
layout: center
---

<div class="rollout-joke-setup">the secret to coordinating ordered rollouts at scale</div>

<!--
When we talk about breaking changes, we're usually talking about clients and servers. You've probably babysat many annoying changes of the flavor "let's expand what the server accepts first, then update the client to send the new thing, then hopefully remember to re-constrain the server to only accept the new thing. Usually we forget to do the third step. And you have to make sure that you remember to roll out the services in the right order. So, I'm going to begin this talk by telling you the secret to coordinating ordered rollouts in a system the size of OpenAI's:

[30s]
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-word">give up</div>
</div>

<!--
You should give up! Don't give up on correctness, mind you – but if you have a change in trunk that will only work if services are deployed in a certain order, you're going to have a bad time.

[15s]
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase emphasis-phrase-coral">don't rely on humans</div>
</div>

<!--
You shouldn't rely on humans - or agents, for that matter - to catch breaking changes in an unconstrained system let alone babysit a reasonable rollout order. It's too complicated to reason about, it makes rollbacks unsafe, and there are sometimes even circular dependencies that make it entirely impossible. We need a better solution than a human manually gating rollouts to make sure their breaking change gets into production without errors, and we need better ways to constrain the behavior at abstraction boundaries to even make this tractable in the first place.

[27s]
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

Let's imagine I want to add a new field to this schema, eye-color. All future users are going to set this field, so we want to make it required. 

(s)

Oops, it rolled out to the reader first! eye_color is required, so we're seeing some errors. It's ok, let's roll out the writer too. 

(s, p)

Look at this - even if all the readers and all the writers had flipped at exactly the same time, inflight requests would have failed. 

It's only now that the reader and the writer have been at the same version for a while (all the queues and RPC have drained) will we stop seeing errors.

[1m]
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
OK, you're thinking. We solved this problem in like 2001 with protobufs. Why is robbie up there complaining about a solved problem? surely someone told him about protos before he got up here and embarrassed himself.

Well, yes. Protos do indeed solve the wire compatibility problem. But they do so by substantially weakening the set of states they can represent. Protos, what with their optional-only semantics and no logic constraints -  make wire compatibility easy - but you're sacrificing constraints in your application for ease of wire compat. If you want your systems to have these solid abstraction boundaries - which I very much do - your invariants belong at the boundaries of your systems. Your application code should not be in the business of dealing with old versions of stuff forever, leaving dead branches of logic that you can never prune, and generally abandoning service developers to a situation where they have to handle all possible sets of states from all time. 

So when I talk about defining schemas, I'm not just talking about the wire format. I'm also talking about contracts that can encode powerful rules, like json schema or extensions to protos like protovalidate.

Unfortunately, though, of course, the stricter your schemas are, the easier it is for you to make a so-called "breaking change."

[1:10]
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
Our industry's current tooling encourages what I call optionalslop. 

Every migration, rollback path,
and compatibility tail leaves some ugly *residue* in the schema. The schema on the left here shows 
a type that is technically compatible but increasingly bad at expressing which states are actually valid
today. This type doesn't make impossible states unrepresentable, and pushes the cleanup burden into business logic everywhere that reads it. So, sure, you're never going to get wire incompatibilities when you're using protos, but instead you'll see weird errors much deeper in your application code when you assume that legacy_full_name can't co-occur with phone_verified or something. If you don't encode as many rules as possible into your contract, I absolutely guarantee that a future developer will take advantage of the flexibility to send something that you don't expect.

I don't want to give you the feeling that I'm anti-proto. Certainly not. It's
just not enough. We need a wire format plus additional rules, because at the
end of the day, only the abstraction boundary is guaranteed. And you'd better
be writing that boundary in some schema definition language so you can generate
code from it that doesn't sneak in any assumptions. 

I promised at the beginning that we can keep things safe while also reducing cognitive overhead and making impossible states impossible. We do this with tooling, but first, i want to make one comment about how this is even more important for agents than it is for humans.

[1:20]
-->

---

# Strict contracts are better for ~~humans~~ agents

<div class="deck-grid-3 mt-8 agent-contract-grid">
  <div class="law-card success">
    <h3>Smaller legal state space</h3>
    <p>Fewer ambiguous shapes for an agent depend on.</p>
  </div>
  <div class="law-card success">
    <h3>Hidden assumptions become explicit</h3>
    <p>Put the rule at the boundary so the agent does not have to recover it.</p>
  </div>
  <div class="law-card success">
    <h3>Crisper test oracle</h3>
    <p>A strict contract allows an agent loop to quickly iterate upon correctness.</p>
  </div>
</div>

<div class="deck-callout mt-8">
  <p class="deck-quote">Agentic workflows get safer when the boundary is narrow enough to make bad states impossible, not just unlikely.</p>
</div>

<!--
Look, I work at OpenAI. I'm pretty AGI-pilled, and I believe strongly that our models are already better than humans at reasoning through most of this stuff, and will be much MUCH better in the near future. But that doesn't obviate the need for strict contracts - in fact, it makes them even more important. 

Today, and I think, forever, agents, like humans will be able to build systems that they themselves cannot fully understand. These systems will be bigger and more impressive than anything I can make, but still too complex to page into memory, so to speak. 

As our models get stronger and stronger, these abstraction boundaries can get bigger and bigger – but so will the underlying systems they support! Until and unless agents stop being able to build systems that they themselves can't fully understand, they need help to verify correctness of each change. We will always need abstraction boundaries with good contracts that are at most the size of an agent's ability to fully understand.

So what should these contracts actually look like? Put as many constraints into your contract as possible. If the business logic
depends on the rule, the rule belongs at the boundary. Then you can generate
types for handlers that fulfill this contract, and you don't have to handle
missing or malformed states deep in application code.

I famously can only hold only one thing in my head, which is why I like types and hate side effects
so much. They let me think about just one part of my system at a time without
worrying that my understanding (or lack thereof) might bleed into other systems. This is good for especially
dumb humans like me, obviously. But I think it's even better for agents.

Models are not great at reconstructing your implicit invariants from a pile of
surrounding code and tribal knowledge. If the boundary is loose, they will
invent plausible-looking states that are subtly wrong. If the contract is
strict, the legal state space is smaller, hidden assumptions become explicit,
and you get a much sharper oracle for CI, review, and most importantly, agentic loops. If an agent can fuzz its own boundary by generating all reasonable input that satisfies some contract, it's going to be much more effective at proving its own correctness and building a robust system.

Don't make your agents or your developers re-derive the contracts every time they look at the codebase. In some cases, they can, but doing so is like writing a runbook when you could have written a script. The script is just going to work. The runbook is going to mostly work, get out of date, have subtle bugs, etc. Even if context length becomes free, this will continue to be true. In terms of minimizing cognitive overhead for humans or for agents, a 100% guarantee that is easily definable is worth so much.

[2:30]
-->

---

# Writers strict. Readers wide.

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
OK, I've been waxing philosophical about how important strict contracts are for like 11 slides. It's time to tell you what we can actually do about it without adding too much friction.

Adding all this strictness requires a change in the way we think about contracts. We need to stop sharing types between client and server, between serializer and deserializer. Delete your common/types library and replace it with a DSL that can generate separate client and server types.

This is the shape I actually want. Writers should be as strict as possible.
They should emit today's contract, not some giant compromise type that has been
weakened by every migration you've ever done. If your business logic requires that age is required for all new users, make it impossible to ever serialize an ageless user again! 

Readers, or deserializers, are where the compatibility burden belongs. They
should accept the union of the last few writer versions, because that's where
skew lands in practice. 

I know everyone loves to share types between client and server. Of course
they do. It feels simpler, cheaper, cleaner. So if you want people not to do
that, the tooling has to be really good. Separate writer and reader types can't
feel like a tax. It has to feel like the easy path.

[1:10]
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
  <div class="tooling-step"><strong>1</strong><span>Update the schema.</span></div>
  <div class="tooling-step"><strong>2</strong><span>Detect breaking changes.</span></div>
  <div class="tooling-step"><strong>3</strong><span>Keep the writer as strict as possible.</span></div>
  <div class="tooling-step"><strong>4</strong><span>Make readers a tagged union of the last few writers.</span></div>
  <div class="tooling-step"><strong>5</strong><span>Measure how often old writer branches still deserialize.</span></div>
  <div class="tooling-step"><strong>6</strong><span>Delete old branches once those metrics hit zero.</span></div>
</div>

<!--
So what does that mean mechanically?

The source of truth should be a contract in a schema DSL: proto plus
protovalidate, JSON Schema, whatever you use. You can generate that from code if you insist, but I think maintaining the schema directly is better.

Whenever the schema changes, use static analysis where possible and fuzzing otherwise to ask whether the change is breaking under partial rollout, and in which direction. That is a property of the contract, not of whatever data happens to be flowing today. You don't get to rely on assumptions that aren't encoded in the schema.

If the change is breaking in either direction, CI should complain and tell you to stamp a new type. Writers use only the new type. Readers use the stamped union of the historical writer types they still need to accept. 

If you want to be really fancy, your generated code should make it impossible to serialize from reader types or deserialize from writer types. Make impossible states impossible, right!

If you've done this right, a few nice things happen. Engineers stop simulating cross-version breakage in their heads; CI does that part. 

Schema updates become more mechanical. And schemas start to represent points in time, which means you can branch explicitly on the stamp and later delete old branches when the metrics say they're gone. This is a superpower, because it allows us to delete old code that we can guarantee is no longer used. 

Let's reiterate the 6 step process.

1. Update the schema to express some new expectation in business logic. "All users have names."
2. Detect breaking changes. CI should look at your schema vs the one on trunk, master, main, whatever you call it, and reject changes that it can statically, or via fuzzing, determine are unsafe.
3. Generate serializer and deserializers in the languages of your choice. Keep the writer type, the serializer type, as strict as possible. If all users have names, don't let name be optional.
4. Make reader, or deserializer types, a tagged union of the last few writer types. This allows us to explicitly branch at previous versions of your schema, basically quarantining and then eventually deleting old bits of unmaintained code.
5. Measure how often old readers are still used. Since we're generating all these deserializers, why not generate them with some standard telemetry!
6. Delete old branches when telemetry nears zero. 

This requires a lot of tooling. You need a complicated breaking change detector. The logic is no longer nearly as simple as proto breaking changes. You need telemetry. You need code generation from your DSL into your programming language. You need to be able to fetch old schema versions CI, and you need a way to mark schemas as currently evolving so you don't block engineers who don't care if they're making breaking changes to a brand new type. But once you have all that, it feels magic. Your generated clients just work, and more importantly, your business logic gets so much simpler - you just match on each part of the union you still have to support, and implement simple, constrained logic per-branch rather than an optional soup.

I promise I'll get to some of this tooling, but let's use the same animation as before to walk through a simple example. 

[2:50]
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
For most rollouts where the change isn't breaking, CI will just let the change through and you don't have to do anything fancy. But let's take the example of the most verboten type of change: changing the type of a field. 

This change is obviously breaking in both directions - old readers can't understand new writers and vice versa, and I still don't recommend it. But it's doable. In this case, we're going to change `interests` from a list of strings to an int.

First, CI sees that you're trying to deploy a breaking change. It doesn't let you update the writer type yet, since the reader type on master doesn't accept v5. Instead, you run `stamp` and now all the readers accept either the old version or the new version.

(s)

Then, when metrics show that all readers are rolled out, CI finally lets you merge the new, strict, writer type:

(s)s

And only after the old data tail is gone do you remove the old
branch. 

In some cases, you'll never be able to remove the old types, but that's ok! It's much cleaner to have different codepaths based on a union branch.

I don't really like talking about forward and backward compatibility since it always confuses me from whose perspective? For example, a system that takes input and returns output simultaneously acts as a deserializer (when it takes its input) and a serializer (when it returns its output.) So, instead, I prefer to talk about breaking changes from the perspective of the writer and reader, or serializer and deserializer.

[1:25]
-->

---
layout: center
---

<div class="emphasis-slide">
  <div class="emphasis-phrase">Tooling!</div>
</div>

<!--
The first two thirds of this talk were kind of philosophical; like "in a perfect world, here's how I'd think about breaking changes. The thing that makes this hard is that it's very difficult to detect breaking changes in a sufficiently powerful contract language. The set of breaking changes rules for protos are really easy. The set of breaking changes rules for JSON schema are much much harder, precisely because they are more expressive. Harder enough that I'm not aware of any open source project that does this in any reasonable way, even though I think it would be useful industry-wide, for everyone who uses JSON!

[30]
-->

---
class: demo-full-bleed
---

<CheckerEmbed />

<!--
Until today, that is! I've been frustrated by this for years and finally took the time to sit down to try to write a generic JSON schema subsumption checker. We can statically analyze arbitrary JSON schemas and try to detect if they're breaking.

[15]
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

<div class="deck-callout mt-4">
  <p class="deck-quote">A schema change is compatible in a direction exactly when every value accepted before is still accepted after, or vice versa.</p>
</div>

<!--
Let me formalize what I mean by subsumption checker really quick. The core question is just set containment.

Think of a schema as denoting a language of valid JSON values, L(schema).

Then a subsumption checker asks whether one language is a subset of the other.

If L(new) is a subset of L(old), that is, all values valid under the new schema are valid under the old schema, a new writer is safe for an old reader.
If L(old) is a subset of L(new), that is, all values valid under the old schema are valid under the new schema, an old writer is safe for a new reader.

When either relation fails, if possible, the checker should able to produce a witness value showing the difference.

The reason this is hard is that it's impossible in most cases to enumerate the actual language, so you can't do an actual subset check. Instead, a subsumption checker like jsoncompat has to do special logic for each keyword that proves properties of new and old schemas. This gets really hard with recursive schemas, sum types, and other arbitrary constraints like regex format strings.

The important point here is that the only input to the language is the schema itself. If your business logic assumes some invariant that is not expressible in the schema, the subsumption checker cannot possibly catch it. So, you should try to avoid assuming such invariants, or, if you have to, you must extend the schema DSL itself.

[1:15]
-->

---
class: demo-full-bleed
---

<CheckerEmbed />

<!--
Now, let's do a brief live demo. This is actually jsoncompat.com iframed into this presentation, we'll see if it works. The actual subsumption checker is written in rust but compiled to wasm for javascript usage. 

Let's take a look at this simple schema on the right. We've got an object type with two fields: `name` and `age`. They both have some additional constraints, like minLength for the string or minimum for the age. 

First let's check compatibility between this schema and itself. 

Great, we've done so, and we've generated some representative sample data at the bottom as well.

Now, let's tighten the new schema. Let's make minLength 6. If you think about it for a second, all values representable by the new schema are representable by the old schema, but names with length 5 are only valid under the old schema but not the new schema. This is a breaking change for the `deserializer` role because there is old data that the serializer can emit that is no longer valid under the new schema!

I think this is pretty cool, and you can run this against all schemas in your repo quite quickly. 

This static checker works for almost all schemas, so fuzzing is rarely necessary. 

However for a sufficiently powerful constraint language, not every compatibility
question is that easy. The JSON schema "not" keyword comes to mind; it's really hard to detect statically whether something does not fulfull any instance of a schema, or a complicated format or whatever. Once you have richer combinations of conditionals,
cross-field constraints, and schema composition, there are changes where a
complete static answer is much harder or not practical.

[Click on fuzzer]

So then we need fuzzing too. The workflow is: prove what we can statically, and
when the checker can't fully decide, search for concrete counterexamples. I
like that combination a lot, because the static checker gives you fast,
deterministic guardrails, and fuzzing gives you real examples when the logic
gets too expressive for a complete proof. The nice thing about having the fuzzer is it also lets me and my agents iterate a lot faster on the static side of the house. The test harness is basically the fuzzer.

[play with the fuzzer a bit]

This is MIT-licensed and it's the first time I'm talking about it anywhere so I'm sure it has lots of bugs, but so far where we've used it, it's been a huge boon for catching breaking changes at our storage boundaries.

[2:20]
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
Instead of telling you that you have to go back to your company and insist that everyone rewrites all of their storage logic separating all of our serializer and deserializer types, write some codegen, etc, I want to leave you with a suggestion as to how to slowly adopt tooling like this. 

At openai, we have a lot of Pydantic schemas that define something we're storing somewhere; maybe in a database, maybe in redis, whatever. We don't have to go all the way off the bat and split the reader and writer type, even if I wish we did! Instead, we can decorate all these types with a jsoncompat decorator that writes the current schema to disk and checks breaking changes against previous commits, using the same static checks and fuzzing I just told you! While this isn't as powerful as schema-first design and generated code, it's already caught a ton of subtle cases where people didn't realize they were making breaking changes.

Here, it's trivial to adopt. Put the compatibility policy right next to the type definition. If we're using it for both directions, we say "both." The stable ID
is the durable identity for this contract across renames and refactors, so CI
can compare the current schema against the historical snapshots for that same logical payload.

With `direction="both"`, a change has to be safe for old readers seeing new
writes and for new readers seeing old writes. Eventually, I hope that we can split UserProfile here into a writer and reader type, and evolve them more independently, allowing us to have strict writers and union readers like I keep talking about.

[1:15]
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
So if I were going to summarize this as an SRE-ish playbook, it would be these
four things.

Constrain: make strict schemas the default, so hidden assumptions become
contract rules instead of tribal knowledge.

Split: generate separate reader and writer types in whatever language your
service uses, from the schema, and make the historical reader union cheap to
maintain.

Gate: run CI against the schema itself, and against previous versions of that
schema, so you can detect breaking changes mechanically before they merge.

Observe: measure which old versions are still being read, so you can see the
tail, understand rollback risk, and know when cleanup is actually safe.

That's the durable version of this. Not a one-off preflight for one scary
change, not a system that requires engineers to think about subsumption every day, and think about which direction things are getting serialized and deserialized, but a system that makes the safe path the normal path.

[1:00]
-->

---
layout: center
---

<div class="thanks-slide">
  <div class="thanks-title">Questions?</div>
  <a class="thanks-link" href="https://jsoncompat.com">slides and tooling at jsoncompat.com</a>
</div>

<!--
Thanks so much, I think I now have some time for questions. I haven't uploaded these slides to jsoncompat.com yet but I will once I send them to the conference organizers.

[15]
-->
