import { useState } from "react";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/fuzzer")({
  component: FuzzerPage,
});

let wasmReady: Promise<any> | null = null;

async function loadWasm() {
  if (!wasmReady) {
    wasmReady = import("/jsoncompat_wasm/jsoncompat_wasm.js").then(async (m) => {
      // @ts-ignore – wasm-pack default export exists
      await (m.default as unknown as () => Promise<void>)();
      return m;
    });
  }
  return wasmReady;
}

function FuzzerPage() {
  const [schema, setSchema] = useState("{\n  \"type\": \"string\"\n}");
  const [depth, setDepth] = useState(5);
  const [value, setValue] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function runGenerate() {
    setError(null);
    setValue(null);
    try {
      const m = await loadWasm();
      const v = await m.generate_value(schema, depth);
      setValue(v);
    } catch (err) {
      setError((err as Error).message ?? String(err));
    }
  }

  return (
    <main className="mx-auto max-w-3xl px-4 py-8">
      <h1 className="mb-4 text-3xl font-bold">Value generator / schema fuzzer</h1>

      <label className="mb-2 block font-medium">Schema</label>
      <textarea
        className="h-64 w-full rounded-md border border-gray-300 p-2 font-mono text-sm"
        value={schema}
        onChange={(e) => setSchema(e.target.value)}
      />

      <div className="mt-4 flex items-center space-x-4">
        <label htmlFor="depth" className="font-medium">
          Depth:
        </label>
        <input
          id="depth"
          type="number"
          min="1"
          max="10"
          value={depth}
          onChange={(e) => setDepth(Number(e.target.value))}
          className="w-16 rounded-md border border-gray-300 p-1 text-right"
        />

        <button
          onClick={runGenerate}
          className="rounded bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700"
        >
          Generate
        </button>
      </div>

      {value && (
        <div className="mt-6">
          <h2 className="mb-2 text-xl font-semibold">Sample value</h2>
          <pre className="rounded-md bg-gray-100 p-4 text-sm overflow-auto">
            {value}
          </pre>
        </div>
      )}

      {error && <p className="mt-4 text-red-600">{error}</p>}
    </main>
  );
}
