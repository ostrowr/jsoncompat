import { useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import init, { check_compat } from "jsoncompat";

export const Route = createFileRoute("/checker")({
  component: CheckerPage,
});

function CheckerPage() {
  const [oldSchema, setOldSchema] = useState('{\n  "type": "string"\n}');
  const [newSchema, setNewSchema] = useState('{\n  "type": "number"\n}');
  const [role, setRole] = useState("both");
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function runCheck() {
    setError(null);
    setResult(null);
    try {
      await init(); // TODO: only do once
      const ok = await check_compat(oldSchema, newSchema, role);
      setResult(ok ? "✔ Compatible" : "✖ Incompatible");
    } catch (err) {
      setError((err as Error).message ?? String(err));
    }
  }

  return (
    <main className="mx-auto max-w-4xl px-4 py-8">
      <h1 className="mb-4 text-3xl font-bold">Schema compatibility checker</h1>

      <div className="grid gap-6 md:grid-cols-2">
        <div>
          <label className="mb-2 block font-medium">Old schema</label>
          <textarea
            className="h-64 w-full rounded-md border border-gray-300 p-2 font-mono text-sm"
            value={oldSchema}
            onChange={(e) => setOldSchema(e.target.value)}
          />
        </div>
        <div>
          <label className="mb-2 block font-medium">New schema</label>
          <textarea
            className="h-64 w-full rounded-md border border-gray-300 p-2 font-mono text-sm"
            value={newSchema}
            onChange={(e) => setNewSchema(e.target.value)}
          />
        </div>
      </div>

      <div className="mt-4 flex items-center space-x-4">
        <label className="font-medium" htmlFor="role-select">
          Role:
        </label>
        <select
          id="role-select"
          value={role}
          onChange={(e) => setRole(e.target.value)}
          className="rounded-md border border-gray-300 p-1"
        >
          <option value="serializer">serializer</option>
          <option value="deserializer">deserializer</option>
          <option value="both">both</option>
        </select>

        <button
          onClick={runCheck}
          className="rounded bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700"
        >
          Check
        </button>
      </div>

      {result && <p className="mt-4 text-xl font-semibold">{result}</p>}
      {error && <p className="mt-4 text-red-600">{error}</p>}
    </main>
  );
}
