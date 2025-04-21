import { createFileRoute } from "@tanstack/react-router";
import init, { check_compat, generate_value } from "jsoncompat";
// Import the raw wasm asset so Vite gives us the final URL it will be served at.
// The `?url` suffix tells Vite to return the URL string instead of inlining / compiling it.
// Using this explicit URL avoids any ambiguity about where the runtime should fetch the
// binary from (and works in dev, preview and any static‑hosted production build).
// eslint-disable-next-line import/no-unresolved
import wasmUrl from "jsoncompat/jsoncompat_wasm_bg.wasm?url";
import { useState } from "react";

const INITAL_OLD_SCHEMA = `{
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}`;

const INITAL_NEW_SCHEMA = `{
  "type": "object",
  "properties": {
    "name": { "type": "string", "minLength": 5 },
    "age": { "type": "integer", "minimum": 18 }
  }
}`;

export const Route = createFileRoute("/checker")({
  component: CheckerPage,
});

function CheckerPage() {
  const [oldSchema, setOldSchema] = useState(INITAL_OLD_SCHEMA);
  const [newSchema, setNewSchema] = useState(INITAL_NEW_SCHEMA);
  const [compat, setCompat] = useState<Record<string, boolean> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [exampleOld, setExampleOld] = useState<string | null>(null);
  const [exampleNew, setExampleNew] = useState<string | null>(null);

  async function runCheck() {
    setError(null);
    setCompat(null);
    setExampleOld(null);
    setExampleNew(null);
    try {
      await init(wasmUrl); // TODO: only do once

      const roles = ["serializer", "deserializer", "both"] as const;
      const results: Record<string, boolean> = {} as Record<string, boolean>;
      for (const r of roles) {
        results[r] = await check_compat(oldSchema, newSchema, r);
      }
      setCompat(results);

      // Generate illustrative examples for both schemas
      try {
        setExampleOld(await generate_value(oldSchema, 5));
      } catch (_) {
        setExampleOld(null);
      }
      try {
        setExampleNew(await generate_value(newSchema, 5));
      } catch (_) {
        setExampleNew(null);
      }
    } catch (err) {
      setError((err as Error).message ?? String(err));
    }
  }

  return (
    <main className="mx-auto max-w-4xl px-4 py-8 space-y-6">
      <h1 className="mb-4 text-3xl font-bold">Schema compatibility checker</h1>

      <div className="grid gap-6 md:grid-cols-2">
        <div>
          <label htmlFor="old-schema" className="mb-2 block font-medium">
            Old schema
          </label>
          <textarea
            id="old-schema"
            className="h-64 w-full rounded-md border border-gray-300 p-2 font-mono text-sm"
            value={oldSchema}
            onChange={(e) => setOldSchema(e.target.value)}
          />
        </div>
        <div>
          <label htmlFor="new-schema" className="mb-2 block font-medium">
            New schema
          </label>
          <textarea
            id="new-schema"
            className="h-64 w-full rounded-md border border-gray-300 p-2 font-mono text-sm"
            value={newSchema}
            onChange={(e) => setNewSchema(e.target.value)}
          />
        </div>
      </div>

      <div className="mt-4">
        <button
          type="button"
          onClick={runCheck}
          className="rounded bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700"
        >
          Check compatibility
        </button>
      </div>

      {/* Compatibility explainer */}
      <section className="mt-10 rounded-lg overflow-hidden shadow ring-1 ring-gray-200 max-w-full text-sm overflow-x-auto">
        <h2 className="bg-gray-50 px-4 py-3 text-base font-semibold text-gray-900 border-b border-gray-200">
          What counts as a <em>compatible</em> change?
        </h2>
        <table className="min-w-max w-full border-collapse">
          <thead>
            <tr className="bg-gray-100 text-gray-900">
              <th className="px-4 py-2 text-left whitespace-nowrap">Role</th>
              <th className="px-4 py-2 text-left">Compatibility rule</th>
              <th className="px-4 py-2 text-left">Status</th>
            </tr>
          </thead>
          <tbody className="align-top">
            <tr className="bg-blue-50">
              <td className="px-4 py-3 whitespace-nowrap font-medium align-top">
                🖊️ Serializer
              </td>
              <td className="px-4 py-3 align-top">
                Every value produced with the <em>new</em> schema must also
                satisfy the <em>old</em> schema.
              </td>
              <td
                className={`px-4 py-3 align-top font-semibold ${compat == null ? "text-gray-400" : compat.serializer ? "text-green-700" : "text-red-700"}`}
              >
                {compat ? (compat.serializer ? "✔" : "✖") : "—"}
              </td>
            </tr>
            <tr className="bg-amber-50">
              <td className="px-4 py-3 whitespace-nowrap font-medium align-top">
                👓 Deserializer
              </td>
              <td className="px-4 py-3 align-top">
                Every value valid under the <em>old</em> schema must also
                satisfy the <em>new</em> schema.
              </td>
              <td
                className={`px-4 py-3 align-top font-semibold ${compat == null ? "text-gray-400" : compat.deserializer ? "text-green-700" : "text-red-700"}`}
              >
                {compat ? (compat.deserializer ? "✔" : "✖") : "—"}
              </td>
            </tr>
            <tr className="bg-purple-50">
              <td className="px-4 py-3 whitespace-nowrap font-medium align-top">
                🔄 Both
              </td>
              <td className="px-4 py-3 align-top">
                Both serializer <em>and</em> deserializer guarantees must hold.
              </td>
              <td
                className={`px-4 py-3 align-top font-semibold ${compat == null ? "text-gray-400" : compat.both ? "text-green-700" : "text-red-700"}`}
              >
                {compat ? (compat.both ? "✔" : "✖") : "—"}
              </td>
            </tr>
          </tbody>
        </table>
      </section>

      {/* explanatory list removed – table is now self‑contained */}

      {(exampleOld || exampleNew) && (
        <div className="mt-8 grid gap-4 md:grid-cols-2">
          {exampleOld && (
            <div>
              <h3 className="mb-2 font-medium">Old schema example</h3>
              <pre className="overflow-auto rounded-md bg-gray-100 p-4 text-sm">
                {exampleOld}
              </pre>
            </div>
          )}
          {exampleNew && (
            <div>
              <h3 className="mb-2 font-medium">New schema example</h3>
              <pre className="overflow-auto rounded-md bg-gray-100 p-4 text-sm">
                {exampleNew}
              </pre>
            </div>
          )}
        </div>
      )}

      {error && <p className="mt-4 text-red-600">{error}</p>}
    </main>
  );
}
