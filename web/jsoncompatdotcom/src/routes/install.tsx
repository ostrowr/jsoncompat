import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/install")({
	component: InstallPage,
});

function InstallPage() {
	return (
		<main className="mx-auto max-w-4xl px-4 py-8 space-y-8">
			<h1 className="text-3xl font-bold">Installation & usage</h1>

			<section>
				<h2 className="mb-2 text-xl font-semibold">Rust</h2>
				<p className="mb-2">
					Add the crate to your <code className="font-mono">Cargo.toml</code>:
				</p>
				<pre className="rounded-md bg-gray-100 p-4">
					<code>[dependencies] jsoncompat = "*"</code>
				</pre>
			</section>

			<section>
				<h2 className="mb-2 text-xl font-semibold">Python</h2>
				<p className="mb-2">Install from PyPI:</p>
				<pre className="rounded-md bg-gray-100 p-4">
					<code>pip install jsoncompat</code>
				</pre>
				<p className="mt-2">Then:</p>
				<pre className="rounded-md bg-gray-100 p-4">
					<code>{`import jsoncompat

old_schema = {"type": "string"}
new_schema = {"type": "number"}

print(jsoncompat.check_compat(old_schema, new_schema, role="both"))`}</code>
				</pre>
			</section>

			<section>
				<h2 className="mb-2 text-xl font-semibold">WebAssembly</h2>
				<p className="mb-2">
					Import the pre-built WASM package in your browser project:
				</p>
				<pre className="rounded-md bg-gray-100 p-4">
					<code>{`import init, { check_compat, generate_value } from "jsoncompat";

await init();
const ok = await check_compat("{...}", "{...}", "both");`}</code>
				</pre>
			</section>

			<section>
				<h2 className="mb-2 text-xl font-semibold">Links</h2>
				<ul className="list-inside list-disc space-y-1">
					<li>
						<a
							className="text-blue-600 hover:underline"
							href="https://github.com/your-org/jsoncompat"
						>
							GitHub repository
						</a>
					</li>
					<li>
						<a
							className="text-blue-600 hover:underline"
							href="https://crates.io/crates/jsoncompat"
						>
							crates.io page
						</a>
					</li>
					<li>
						<a
							className="text-blue-600 hover:underline"
							href="https://pypi.org/project/jsoncompat/"
						>
							PyPI package
						</a>
					</li>
				</ul>
			</section>
		</main>
	);
}
