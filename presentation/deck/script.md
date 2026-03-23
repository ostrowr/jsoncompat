# Escaping Version Skew: speaker script

Target length: 35 minutes total.

Pacing assumption:
- 24-27 minutes of spoken material
- 5-7 minutes across the two simulator beats
- 3-5 minutes for the checker demo, including one reset/retry if needed

The standalone emphasis slides are there to create room. Do not rush them. Land the sentence, pause, advance.

## Demo Runbook

Use this sequence on [SLIDE 16] Checker.

Before the talk:
- Have both the old and new schemas pre-staged in the iframe or in a local note so you are not typing a long object live.
- The point of the demo is one static incompatibility result, not a product tour.

Old schema to use:

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

New schema to use:
- Use the same schema, but change `"exclusiveMaximum": 5` to `4`.

Live sequence:
1. If needed, paste the old schema and the new schema into the checker.
2. Run the compatibility check for the rollout direction that matters here: old writer, new reader.
3. Say: "This is the kind of break that looks tiny in review and obvious to a checker."
4. Say: "We don't need to hope a reviewer mentally simulates the overlap."
5. Point to `4` as the witness: old valid state, new invalid state.

What to say on the result:
- "For this change, incompatibility is not empirical. It's derivable from the contracts."
- "Old writers may still emit `4` while new readers are live."
- "`4` is the witness: old writers can still emit it, new readers reject it."

If the iframe is not cooperating, skip any live editing and show the pre-staged old/new pair. The static incompatibility result is the whole point.

## Script

[SLIDE 1] Title hero: red failures in the network

"From far away, system diagrams look legible. Data moves. Requests route. Boxes light up. It feels like if you understand the shape, you understand the behavior.

But real systems are not just topology. They are topology plus time. They are multiple versions, partial deploys, retries, queues, caches, and state that outlives the code that wrote it.

So the question for this talk is not only whether a system is correct in steady state. The question is what happens while it is changing."

"That is the axis a lot of our diagrams hide. They show structure. They do not show coexistence.

And coexistence is where a lot of compatibility bugs actually live."

[SLIDE 2] Audience question: "Errors are climbing during a deploy"

"Before I tell you what we did, I want to ask the room the operational question first.

You are in the middle of a deploy. The new version is taking more traffic. Errors are climbing. What do you do?"

[SLIDE 3] Incident: "A mixed fleet shared one cache"

"Let me make this concrete immediately.

We had a mixed fleet sharing one cache. During a deploy, newer pods started writing a new cache format. Older pods were still around, reading from that same cache, and some of them failed to parse what the newer pods had written."

"The bug was not just in the new code. The bug was in the interaction between versions, through shared state. A locally reasonable change met a very ordinary deployment reality."

"The cache connected them. Shared state turns version skew from a local code concern into a system concern, because one version can write a value that another version will only discover later, on a different request."

[SLIDE 4] Same incident: "Rollback increased errors"

"The worst part was this: rollback increased errors.

Old readers came back while bad cached data was still alive."

"As the deploy progressed, more new pods were writing the new format, so old pods failed more often. Then as old pods disappeared, errors fell. During rollback, we reintroduced the old readers, and they started hitting the incompatible cached data again."

"This failed because one live version wrote a state that another still-live version could not accept.

That sentence is the bridge to the rest of the talk. I want to stop treating this as a one-off incident and start treating it as a schema evolution problem over a mixed-version state space."

"And I am going to keep using one tiny value as the villain in this story: `4`.

It is valid under the old rule, invalid under the new one, durable enough to sit in a cache, and exactly the kind of edge value that reappears during rollback."

[SLIDE 5] Joke setup: "the secret to coordinating ordered rollouts at scale"

"So here is the setup.

You want the secret to coordinating ordered rollouts at scale."

[SLIDE 6] Emphasis: "give up"

"Give up.

Not on correctness. Not on safety. On perfect choreography."

"At small scale, maybe you can tell a story like: first all the readers, then all the writers, then wait, then clean up.

At large scale, with retries and caches and rollback and long-lived data, that is not a strategy. That is a wish."

[SLIDE 7] Emphasis: "Don't rely on rollout order."

"That is the first refrain: don't rely on rollout order.

If your safety argument depends on every part of a distributed system changing in exactly the order you imagined, you do not have a very good safety argument."

[SLIDE 8] Minimal mechanics demo

"This is the simplified subsystem picture most of us reason from.

One sender version. One receiver version. One message shape. Everything in place."

"It is a useful model. It is just not the hard part.

The hard part is the period where the system is neither old nor new. It is both."

"This is one animation, not two slides. It starts in steady state, and then I can step it into broken overlap."

[press s]

"Now one side changes. Not the whole world. One side.

And notice what did not happen: the old world did not disappear. Old packets are still in flight. Old code is still alive. Some messages were produced under old rules. Some will be consumed under new ones."

[press s]

"Now we have the other side of the transition.

One tiny schema diff turns into two different compatibility questions depending on direction. Can the new reader handle old data? Can the old reader handle new data?"

"Those are not the same question. During rollout, both can matter at once.

So whenever someone says a change is compatible, I want to ask a follow-up: compatible for whom, in which direction, during what overlap?"

[SLIDE 9] Boundary: "Parseable is not enough"

"A disciplined wire format solves a real problem. I am not arguing against that.

But transport compatibility can still admit states your logic cannot handle."

"Grammar defines shape. Validation defines state.

A grammar-based schema tells you what can be decoded. A rule layer tells you what your system is actually willing to accept. If the application logic depends on the stronger rule, that rule belongs at the boundary."

"And changing that rule is also a compatibility change under skew.

Tightening a validator is a reader narrow. Loosening one is a reader widen. If writers start emitting values old validators reject, you are back in the same rollout problem."

[SLIDE 10] Mental model: "Compatibility is about sets of states"

"Here is the mental model I want people to leave with, and I want to spend real time on it.

A schema change changes a set of states."

"In this talk, `4` is the concrete edge of that set. If old writers can still emit it and new readers no longer accept it, that is the bug."

"If a writer narrows, that is usually safe, because it emits fewer states.
If a writer widens, that is dangerous under skew, because old readers may reject the new values."

"If a reader widens, that is usually safe, because it accepts more historical states.
If a reader narrows, that is dangerous under skew, because old data or old writers may still exist."

"Backward and forward are useful words, but they are incomplete for this talk. They describe parse direction. Rollout safety also depends on emission, overlap, and time.

This is the callback to the incident: the cache bug was not mysterious. It was a writer widening into a world where old readers were still present."

[SLIDE 11] Emphasis: "Only the contract is guaranteed."

"Only the contract is guaranteed."

[SLIDE 12] What to do instead: strict boundary contracts

"So what do we do instead?

Write the boundary as strictly as the logic. Do not say number-ish if you mean integer. If the rule is less than 5, write less than 5 in the contract. If there are only a few valid modes, enumerate them."

"The only invariant you can count on is the one the schema tells you. The more you push into the schema layer, the more bad input you can reject at the boundary before it becomes a weird internal state.

That is the constructive version of this talk: make the mixed-version state space smaller by making impossible states unrepresentable at the boundary."

"And when I say write the real rule, I mean write the rule that makes `4` visibly important. If the contract says `< 5`, then `4` is not trivia. It is the top edge of the legal state space."

[SLIDE 13] AI agents: "Strict contracts are better for agents"

"I want to linger here, because this is not just a human readability argument.

It matters even more for agentic workflows."

"Agents are bad at recovering hidden assumptions across abstraction boundaries. If the rule only exists in prose, in examples, or in someone's head, an agent will eventually walk through the gap.

A strict contract gives the agent a smaller legal state space, fewer ambiguous shapes to invent, and a much sharper test oracle."

"That is why I do not think the agent point is a side note. It is the same argument as the rest of the talk, just with less slack for ambiguity.

The boundary should be narrow enough that bad states are impossible, not merely unlikely."

[SLIDE 14] Fullscreen union rollout simulator: "Widen the reader first"

"If we cannot rely on perfect rollout ordering, what is the positive pattern?

Same stage, but now the safe version: widen the reader first."

[press s]

"First deploy a reader that can parse both the old and the new shape. In this example, that means a union: v4 or v5.

That gives you a safe overlap period. Old data still works. New data will work once it starts appearing."

[press s]

"Then move the writer to the new shape.

The system is still safe, because the deployed reader population already knows how to parse both."

[press s]

"Only then, when the old tail is gone, remove the old branch.

And if the old tail never really goes away, I would rather keep a discriminated union of past types than collapse everything into one weak type that tries to express every era at once."

[SLIDE 15] Emphasis: "Check it mechanically."

"Check it mechanically."

[SLIDE 16] Checker

"Here is the kind of break code review misses.

Not because reviewers are careless. Because the thing we ask them to do is simulate a distributed, partially upgraded system in their head. That is not a reasonable review burden."

"This is the checker. I am going to use it in the simplest possible way: compare old and new contracts, and let the checker tell us this is unsafe under skew."

Demo steps:
1. Load the old schema and the new schema.
2. Run the check for old writer to new reader.
3. Show the static incompatibility result.

Say:
"This is the kind of break that looks tiny in review and obvious to a checker. We don't need to hope a reviewer mentally simulates the overlap. For this change, incompatibility is not empirical. It's derivable from the contracts. `4` is the witness: old writers can still emit it, new readers reject it."

[SLIDE 17] Tooling: "Writers only emit what readers can parse"

"This is the workflow I want.

First, detect breaking changes mechanically. Prove what you can statically. Use fuzzing as a fallback when the checker cannot fully decide, or when you want concrete counterexamples."

"Second, generate separate local types from one contract. The writer should only emit what the reader side promises to accept.

Third, if a writer change would break skew safety, make the branch explicit in the reader union instead of letting the writer silently start emitting unreadable states."

"If old readers cannot parse it, the writer change is forbidden.

And more generally: do not ask reviewers to simulate a distributed system in their head. Humans should review intent and design. Tools should check the cross-version consequences."

[SLIDE 18] Final implication: "One contract. Two local types."

"There is one more implication.

Serializer and deserializer compatibility are asymmetric during a partial rollout. A serializer wants a narrower output set. A deserializer wants a wider accepted input set."

"If one runtime type serves both, you usually end up with optional soup: the union of rollout-era compromises, not the real domain model.

That ugly type is what I mean by optional soup.

If old readers never go away, I would rather keep a discriminated union of past types than weaken one type until it means everything. That is less aesthetically tidy. It is more honest."

[SLIDE 19] SRE playbook

"So I do not want to end with a preflight checklist for one schema change.

I want to end with what SREs can build so that all subsequent schema changes at their company are safer by default."

"Constrain. Split. Gate. Observe.

Constrain first. Move as much validation as you can to the schema boundary, so hidden assumptions become contract rules instead of tribal knowledge.

Split the local types. Give people tooling that splits reader and writer types by default, and makes historical unions cheap to maintain.

Gate schema diffs in CI. Generate reader and writer contracts, check compatibility mechanically, and fail unsafe changes before merge.

Observe version skew in production. Measure deserializations by payload version so you can see old tails, rollback risk, and when a branch is really gone."

"The goal is not just to catch one bad rollout.

The goal is to make unsafe evolution hard to ship, and safe evolution easy to do."

[SLIDE 20] Thank you

"Thank you."
