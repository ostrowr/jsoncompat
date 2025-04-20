import { createFileRoute } from "@tanstack/react-router";
import init, { check_compat, generate_value } from "jsoncompat";
// Import the raw wasm asset so Vite gives us the final URL it will be served at.
// The `?url` suffix tells Vite to return the URL string instead of inlining / compiling it.
// Using this explicit URL avoids any ambiguity about where the runtime should fetch the
// binary from (and works in dev, preview and any static‑hosted production build).
// eslint-disable-next-line import/no-unresolved
import wasmUrl from "jsoncompat/jsoncompat_wasm_bg.wasm?url";
import { useState } from "react";

export const Route = createFileRoute("/checker")({
	component: CheckerPage,
});

function CheckerPage() {
	const [oldSchema, setOldSchema] = useState('{\n  "type": "string"\n}');
	const [newSchema, setNewSchema] = useState('{\n  "type": "number"\n}');
	const [role, setRole] = useState("both");
	const [result, setResult] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [example, setExample] = useState<string | null>(null);

	async function runCheck() {
		setError(null);
		setResult(null);
		setExample(null);
		try {
			await init(wasmUrl); // TODO: only do once
			const ok = await check_compat(oldSchema, newSchema, role);
			setResult(ok ? "compatible" : "incompatible");

			// Try to generate an illustrative example based on the *new* schema.
			try {
				const ex = await generate_value(newSchema, 5);
				setExample(ex);
			} catch (_) {
				// ignore if generation fails
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
					type="button"
					onClick={runCheck}
					className="rounded bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700"
				>
					Check
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
						</tr>
					</thead>
					<tbody className="align-top">
						<tr className={`${role==='serializer'?"ring-2 ring-blue-400":""} bg-blue-50`}>
							<td className="px-4 py-3 whitespace-nowrap font-medium align-top">🖊️ Serializer</td>
							<td className="px-4 py-3 align-top">Every value produced with the <em>new</em> schema must also satisfy the <em>old</em> schema.</td>
						</tr>
						<tr className={`${role==='deserializer'?"ring-2 ring-amber-400":""} bg-amber-50`}>
							<td className="px-4 py-3 whitespace-nowrap font-medium align-top">👓 Deserializer</td>
							<td className="px-4 py-3 align-top">Every value valid under the <em>old</em> schema must <strong>also</strong> satisfy the <em>new</em> schema.</td>
						</tr>
						<tr className={`${role==='both'?"ring-2 ring-purple-400":""} bg-purple-50`}>
							<td className="px-4 py-3 whitespace-nowrap font-medium align-top">🔄 Both</td>
							<td className="px-4 py-3 align-top">Both serializer <em>and</em> deserializer guarantees must hold.</td>
						</tr>
					</tbody>
				</table>

				
			</section>

			{result && (
				<p
					className={`mt-6 rounded-md px-4 py-3 text-lg font-semibold shadow
            ${result === "compatible" ? "bg-green-50 text-green-800 ring-1 ring-green-300" : "bg-red-50 text-red-800 ring-1 ring-red-300"}
          `}
				>
					{result === "compatible"
						? "✔ Schemas are compatible"
						: "✖ Schemas are NOT compatible"}
				</p>
			)}

			{result && (
				<div className="mt-4 text-sm leading-relaxed text-gray-700 space-y-2">
					<p>
						<strong>Compatible</strong> ⇒ a client that was built against the{" "}
						<em>old</em> schema can still do its job (as&nbsp;
						<code className="font-mono">
							{role === "both" ? "serializer & deserializer" : role}
						</code>
						) when given data that conforms to the <em>new</em> schema.
					</p>
					<ul className="list-inside list-disc space-y-1">
						<li>
							<code className="font-mono">serializer</code> - existing writers
							must produce data that the <em>new</em> schema still considers
							valid.
						</li>
						<li>
							<code className="font-mono">deserializer</code> - existing readers
							must successfully consume data produced under the <em>new</em>{" "}
							schema.
						</li>
						<li>
							<code className="font-mono">both</code> - the two conditions above
							hold simultaneously.
						</li>
					</ul>
				</div>
			)}

			{example && (
				<div className="mt-6">
					<h2 className="mb-2 text-lg font-medium">
						Example value (new schema)
					</h2>
					<pre className="overflow-auto rounded-md bg-gray-100 p-4 text-sm">
						{example}
					</pre>
				</div>
			)}

			{error && <p className="mt-4 text-red-600">{error}</p>}
		</main>
	);
}
