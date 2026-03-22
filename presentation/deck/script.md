# Escaping Version Skew: speaker script

Target length: 35 minutes total.

Pacing assumption:
- 24-27 minutes of spoken material
- 5-7 minutes across the two simulator beats
- 3-5 minutes for the fuzzer demo, including one reset/retry if needed

The simple emphasis slides are there to create room. Do not rush them. Land the sentence, pause, advance.

## Demo Runbook

Use this exact sequence on [SLIDE 19] Fuzzer.

1. Click into the `Schema` textarea.
2. Select all and paste this schema:

```json
{
  "type": "object",
  "properties": {
    "request_id": {
      "type": "string"
    },
    "priority": {
      "type": "integer",
      "minimum": 0,
      "exclusiveMaximum": 5
    },
    "mode": {
      "enum": ["fast", "safe", "bulk"]
    },
    "metadata": {
      "type": "object",
      "properties": {
        "region": {
          "type": "string"
        },
        "attempts": {
          "type": "integer",
          "minimum": 0
        },
        "tags": {
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      },
      "required": ["region", "attempts"]
    },
    "payload": {
      "type": "object",
      "properties": {
        "user_id": {
          "type": "string"
        },
        "city": {
          "type": "array",
          "items": {
            "type": "string"
          }
        },
        "eye_color": {
          "type": ["string", "null"]
        }
      },
      "required": ["user_id", "city"]
    }
  },
  "required": ["request_id", "priority", "mode", "metadata", "payload"]
}
```

3. Set `Depth` to `5`.
4. Set `Examples` to `8`.
5. Click `Generate`.

What to say while it runs:
- "I want a schema that is complex enough to hide edge cases from a human reviewer: nested object, array field, enum, a nullable field, and a bounded integer."
- "The interesting part is not one hand-picked example. It is that the tool can cheaply explore the legal state space and show me values near the edges."
- "If I tighten `priority` from `< 5` to `< 4`, then `4` is the kind of value that used to be valid and now is not. That is exactly the sort of break that looks harmless in review if you do not force yourself to think in sets of states."

Optional second beat if you have time:
1. Change `"exclusiveMaximum": 5` to `4`.
2. Click `Generate` again.
3. Say: "This is what I mean by mechanical checking. I do not want to rely on someone remembering that old writers may still emit the top end of the old range while new readers are live."

If the iframe is already preloaded and you do not want to type live, keep the schema in a local note and paste it from there.

## Script

[SLIDE 1] Title hero: red failures in the network

"From far away, system diagrams look legible. Data moves. Requests route. Boxes light up. It feels like if you understand the shape, you understand the behavior.

But real systems are not just topology. They are topology plus time. They are multiple versions, partial deploys, retries, queues, caches, and state that outlives the code that wrote it.

So the question for this talk is not only whether a system is correct in steady state. The question is what happens while it is changing."

"That is the axis a lot of our diagrams hide. They show structure. They do not show coexistence.

And coexistence is where a lot of compatibility bugs actually live."

"I want to be precise about the scope of the talk. This is not about whether serialization libraries are good or bad. It is about a narrower operational question: when multiple versions of a system overlap in production, what states can move between them, and which of those states are actually safe?"

[SLIDE 2] Incident: "A mixed fleet shared one cache"

"Let me make this concrete immediately.

We had a mixed fleet sharing one cache. During a deploy, newer pods started writing a new cache format. Older pods were still around, reading from that same cache, and some of them failed to parse what the newer pods had written."

"The drawn chart is anonymized, but the shape is real.

The bug was not just in the new code. The bug was in the interaction between versions, through shared state. That is what made it dangerous. A locally reasonable change met a very ordinary deployment reality."

"If you only reviewed the new code in isolation, wrapping a payload with some metadata would not obviously look catastrophic. The production problem came from the fact that old and new were both alive at once, and the cache connected them."

"That last clause is the whole story. The cache connected them. Shared state turns version skew from a local code concern into a system concern, because one version can write a value that another version will only discover later, on a different request."

[SLIDE 3] Same incident: "Rollback increased errors"

"The worst part was this: rollback increased errors.

Old readers came back while bad cached data was still alive."

"As the deploy progressed, more new pods were writing the new format, so old pods failed more often. Then as old pods disappeared, errors fell. During rollback, we reintroduced the old readers, and they started hitting the incompatible cached data again.

So in this case, waiting for the rollout to finish would have caused fewer errors than rolling back."

"That does not mean never roll back. It means rollback is not a free undo once state has already been written.

The moment you have persistence, canaries get much weaker as a protection, because data written by a canary can infect everywhere else."

"This is one reason these incidents feel so disorienting in the moment. You are not debugging one version of the code. You are debugging an interaction among versions, deployment progress, and the lifetime of already-written data."

[SLIDE 4] Joke setup: "the secret to coordinating ordered rollouts at scale"

"So here is the setup.

You want the secret to coordinating ordered rollouts at scale."

"I am choosing those words deliberately, because a lot of operational folk wisdom sounds like that. We talk about sequencing and staging and careful deploy plans as if there is some perfect ritual that makes the problem go away."

[SLIDE 5] Emphasis: "give up"

"Give up.

Not on correctness. Not on safety. On perfect choreography."

"At small scale, maybe you can tell a story like: first all the readers, then all the writers, then wait, then clean up.

At large scale, with retries and caches and rollback and long-lived data, that is not a strategy. That is a wish."

[SLIDE 6] Emphasis: "Don't rely on rollout order."

"That is the first refrain: don't rely on rollout order.

If your safety argument depends on every part of a distributed system changing in exactly the order you imagined, you do not have a very good safety argument."

[SLIDE 7] Steady-state simulator baseline

"This is the simplified subsystem picture most of us reason from.

One sender version. One receiver version. One message shape. Everything in place."

"It is a useful model. It is just not the hard part.

The hard part is the period where the system is neither old nor new. It is both."

"And that period is not an edge case. It is the normal case during deploys, retries, queue drain, cache TTLs, and rollbacks. If our correctness argument only applies after the world has converged, then it skips the part where production spends a lot of its time."

[SLIDE 8] Minimal mechanics demo

"Here is the smallest version of the problem.

We start in steady state."

[press s]

"Now one side changes. Not the whole world. One side.

And notice what did not happen: the old world did not disappear. Old packets are still in flight. Old code is still alive. Some messages were produced under old rules. Some will be consumed under new ones."

[press s]

"Now we have the other side of the transition.

One tiny schema diff turns into two different compatibility questions depending on direction. Can the new reader handle old data? Can the old reader handle new data?"

"Those are not the same question. During rollout, both can matter at once.

A lot of compatibility discussion flattens this into one label. Backward compatible. Forward compatible. Operationally, direction matters because writers create new states and readers decide which states are survivable."

"So whenever someone says a change is compatible, I want to ask a follow-up: compatible for whom, in which direction, during what overlap? The point is that one vague label is hiding several different failure modes."

[SLIDE 9] Emphasis: "Parseable is not enough."

"Parseable is not enough."

[SLIDE 10] Boundary: parseability versus valid state

"A disciplined wire format solves a real problem. I am not arguing against that.

But transport compatibility can still admit states your logic cannot handle."

"Grammar defines shape. Validation defines state.

A grammar-based schema tells you what can be decoded. A rule layer tells you what your system is actually willing to accept. If the application logic depends on the stronger rule, that rule belongs at the boundary."

"And changing that rule is also a compatibility change under skew.

Tightening a validator is a reader narrow. Loosening one is a reader widen. If writers start emitting values old validators reject, you are back in the same rollout problem."

"So a validation layer is necessary, but not sufficient. It gives you a place to put the real invariant. It does not, by itself, tell you how to evolve that invariant safely while old and new versions coexist."

[SLIDE 11] Emphasis: "A schema change changes a set of states."

"A schema change changes a set of states."

[SLIDE 12] Mental model: "Compatibility is about sets of states"

"Here is the mental model I want people to leave with.

If a writer narrows, that is usually safe, because it emits fewer states.
If a writer widens, that is dangerous under skew, because old readers may reject the new values."

"If a reader widens, that is usually safe, because it accepts more historical states.
If a reader narrows, that is dangerous under skew, because old data or old writers may still exist."

"Backward and forward are useful words, but they are incomplete for this talk. They describe parse direction. Rollout safety also depends on emission, overlap, and time.

Once you see it this way, the question gets sharper. Not just 'is this backward compatible?' but compatible in which direction, during what overlap, and for how long?"

"There is good prior art here. Avro separates writer schema from reader schema. Confluent ties compatibility modes to upgrade order. That is coherent when upgrade order is a control surface you really have. My claim is that in many production systems, with partial rollouts and rollback and persistent state, you do not have it reliably enough to make it the center of your safety story."

[SLIDE 13] Emphasis: "Only the contract is guaranteed."

"Only the contract is guaranteed."

[SLIDE 14] What to do instead: strict boundary contracts

"So what do we do instead?

Write the boundary as strictly as the logic. Do not say number-ish if you mean integer. If the rule is less than 5, write less than 5 in the contract. If there are only a few valid modes, enumerate them."

"The only invariant you can count on is the one the schema tells you. The more you push into the schema layer, the more bad input you can reject at the boundary before it becomes a weird internal state.

That is the constructive version of this talk: make the mixed-version state space smaller by making impossible states unrepresentable at the boundary."

"And this is only getting more important in a world of AI agents.

Agents are bad at recovering hidden assumptions across abstraction boundaries. Strict contracts turn those hidden assumptions into machine-checkable boundaries, a smaller legal state space, and a sharper test oracle."

"That helps humans too. But it matters even more when the caller is an agent, because an agent will happily walk through any ambiguity you leave lying around."

"A strict boundary is one of the best gifts you can give an agentic workflow. It reduces hidden context the agent has to infer, and it makes failures crisp enough to turn into tests."

[SLIDE 15] Emphasis: "Widen the reader first."

"Widen the reader first."

[SLIDE 16] Fullscreen union rollout simulator

"If we cannot rely on perfect rollout ordering, what is the positive pattern?

It is this: widen the reader first."

[press s]

"First deploy a reader that can parse both the old and the new shape. In this example, that means a union: v4 or v5.

That gives you a safe overlap period. Old data still works. New data will work once it starts appearing."

[press s]

"Then move the writer to the new shape.

The system is still safe, because the deployed reader population already knows how to parse both."

[press s]

"Only then, when the old tail is gone, remove the old branch.

And if the old tail never really goes away, I would rather keep a discriminated union of past types than collapse everything into one weak type that tries to express every era at once."

"That may look less elegant on paper. In practice, I think it is more honest. It preserves the fact that there were distinct historical shapes, instead of erasing that distinction into one giant maybe-everything type."

[SLIDE 17] Emphasis: "Check it mechanically."

"Check it mechanically."

[SLIDE 18] Demo setup

"Here is the kind of break code review misses.

Not because reviewers are careless. Because the thing we ask them to do is simulate a distributed, partially upgraded system in their head. That is not a reasonable review burden."

[SLIDE 19] Fuzzer

"This is the fuzzer. I am going to use it in the simplest possible way: paste a schema, generate examples, and look at the edge of the state space."

Demo steps:
1. Paste the complex schema from the runbook into `Schema`.
2. Set `Depth` to `5`.
3. Set `Examples` to `8`.
4. Click `Generate`.

While examples render:
"I chose a schema with enough structure to hide bugs from a human: nested object, array, enum, nullable field, and a bounded integer. The point is not that any one example is surprising. The point is that the machine can cheaply explore legal values I might not think to write by hand."

Then, if time:
1. Change `"exclusiveMaximum": 5` to `4`.
2. Click `Generate` again.

Say:
"This is the kind of change that can look tiny in review. But old writers may still emit `4` while new readers are live. If you do not force yourself to think in sets of states over time, it is easy to miss."

"What I am trying to avoid is a process where the reviewer has to be both a domain expert and a model checker. A good demo here is one where the audience can feel how easy it is to miss the top of a range, a nullable corner, or a nested optional field if you are only reading a diff."

[SLIDE 20] Tooling: "Writers only emit what readers can parse"

"This is the workflow I want.

First, detect breaking changes mechanically. Static analysis for the common case, fuzzing for counterexamples where proofs run out."

"Second, generate separate local types from one contract. The writer should only emit what the reader side promises to accept.

Third, if a writer change would break skew safety, make the branch explicit in the reader union instead of letting the writer silently start emitting unreadable states."

"If old readers cannot parse it, the writer change is forbidden.

And more generally: do not ask reviewers to simulate a distributed system in their head. Humans should review intent and design. Tools should check the cross-version consequences."

"That division of labor matters. People are good at deciding whether a change is a good idea. Machines are much better at exhaustively asking, 'What old values still exist? What new values can now be emitted? Which combinations overlap during deploy?'"

[SLIDE 21] Final implication: "One contract. Two generated local types."

"There is one more implication.

Serializer and deserializer compatibility are asymmetric during a partial rollout. A serializer wants a narrower output set. A deserializer wants a wider accepted input set."

"If one runtime type serves both, you usually end up with optional soup: the union of rollout-era compromises, not the real domain model.

If old readers never go away, I would rather keep a discriminated union of past types than weaken one type until it means everything. That is less aesthetically tidy. It is more honest."

"This is the same design instinct as the rest of the talk. Do not smear historical complexity across the whole codebase. Contain it at the boundary, represent it explicitly, and keep the internal model as strong as you can."

[SLIDE 22] Close

"So the compact version is:

Don't rely on rollout order.
Only the contract is guaranteed.
Check it mechanically."

"Assume skew.
Constrain the boundary.
Automate the proof."

[SLIDE 23] Thank you

"Thank you."
