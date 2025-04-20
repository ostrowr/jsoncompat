import { createFileRoute } from "@tanstack/react-router";
import init, { generate_value } from "jsoncompat";
// Get the final URL of the wasm binary from Vite at build time.
// eslint-disable-next-line import/no-unresolved
import wasmUrl from "jsoncompat/jsoncompat_wasm_bg.wasm?url";
import { useState } from "react";

export const Route = createFileRoute("/fuzzer")({
	component: FuzzerPage,
});

function FuzzerPage() {
	const [schema, setSchema] = useState('{\n  "type": "string"\n}');
	const [depth, setDepth] = useState(5);
	const [examples, setExamples] = useState<string[]>([]);
	const [numExamples, setNumExamples] = useState(5);
	const [error, setError] = useState<string | null>(null);

	async function runGenerate() {
		setError(null);
		setExamples([]);
		try {
			await init(wasmUrl); // TODO: only do once
			const vals: string[] = [];
			for (let i = 0; i < numExamples; i++) {
				// eslint-disable-next-line no-await-in-loop
				const v = await generate_value(schema, depth);
				vals.push(v);
			}
			setExamples(vals);
		} catch (err) {
			setError((err as Error).message ?? String(err));
		}
	}

	function copyAll() {
		if (examples.length === 0) return;
		// JSON Lines (JSONL) – one JSON document per line.
		const jsonl = examples.join("\n");
		navigator.clipboard.writeText(jsonl);
	}

	return (
		<main className="mx-auto max-w-3xl px-4 py-8 space-y-6">
			<h1 className="mb-4 text-3xl font-bold">
				Value generator / schema fuzzer
			</h1>

			<label htmlFor="schema" className="mb-2 block font-medium">
				Schema
			</label>
			<textarea
				id="schema"
				className="h-64 w-full rounded-md border border-gray-300 p-2 font-mono text-sm"
				value={schema}
				onChange={(e) => setSchema(e.target.value)}
			/>

			<div className="flex flex-wrap items-end gap-4">
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

				<label htmlFor="num-ex" className="font-medium">
					Examples:
				</label>
				<input
					id="num-ex"
					type="number"
					min="1"
					max="20"
					value={numExamples}
					onChange={(e) => setNumExamples(Number(e.target.value))}
					className="w-16 rounded-md border border-gray-300 p-1 text-right"
				/>

				<button
					type="button"
					onClick={runGenerate}
					className="rounded bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700"
				>
					Generate
				</button>

				{examples.length > 0 && (
					<button
						type="button"
						onClick={copyAll}
						className="rounded border border-gray-300 bg-white px-4 py-2 text-sm shadow-sm hover:bg-gray-50"
					>
						Copy all
					</button>
				)}
			</div>

			{examples.length > 0 && (
				<div className="max-h-[40rem] space-y-4 overflow-y-auto rounded-md border border-gray-200 p-2">
					{examples.map((ex, idx) => (
						<pre
							key={idx}
							className="max-h-40 overflow-auto rounded bg-gray-100 p-4 text-sm"
						>
							{ex}
						</pre>
					))}
				</div>
			)}

			{error && <p className="mt-4 text-red-600">{error}</p>}
		</main>
	);
}
