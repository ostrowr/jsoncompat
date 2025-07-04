import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

export const Route = createFileRoute("/install")({
  component: InstallPage,
});

const tabs = ["CLI", "Rust", "Python", "JavaScript / WASM"] as const;
type Tab = (typeof tabs)[number];

function InstallPage() {
  const [active, setActive] = useState<Tab>("CLI");

  return (
    <main className="mx-auto max-w-4xl px-4 py-8 space-y-8">
      <h1 className="text-3xl font-bold">Installation &amp; usage</h1>

      {/* Tabs */}
      <div className="flex gap-1 border-b border-gray-200">
        {tabs.map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setActive(t)}
            className={`rounded-t px-4 py-2 text-sm font-medium transition-colors
              ${active === t ? "bg-white border-x border-t border-gray-200 -mb-px" : "bg-gray-100 hover:bg-gray-200"}`}
          >
            {t}
          </button>
        ))}
      </div>

      {active === "CLI" && <CliTab />}
      {active === "Rust" && <RustTab />}
      {active === "Python" && <PythonTab />}
      {active === "JavaScript / WASM" && <JsTab />}

      <Links />
    </main>
  );
}

function CodeBlock({ children }: { children: string }) {
  return (
    <pre className="rounded-md bg-gray-100 p-4 text-sm overflow-x-auto">
      {children}
    </pre>
  );
}

function CliTab() {
  return (
    <>
      <section className="space-y-4">
        <p>Cargo (recommended)</p>
        <CodeBlock>cargo install jsoncompat</CodeBlock>
      </section>
      <section className="space-y-4">
        <p>Homebrew</p>
        <CodeBlock>brew install jsoncompat</CodeBlock>
      </section>
      <section className="space-y-4">
        <p>
          or install directly from the{" "}
          <a
            className="text-blue-600 hover:underline"
            href="https://github.com/ostrowr/jsoncompat/releases"
          >
            GitHub releases page
          </a>
        </p>
      </section>
    </>
  );
}

function RustTab() {
  return (
    <section className="space-y-4">
      <p>
        Add the crate to your <code className="font-mono">Cargo.toml</code>
      </p>
      <CodeBlock>[dependencies] jsoncompat = "*"</CodeBlock>
      <CodeBlock>{`use jsoncompat::check_compat;

let old = r#"{"type":"string"}"#;
let new_ = r#"{"type":"number"}"#;

assert!(check_compat(old, new_, "both"));`}</CodeBlock>
    </section>
  );
}

function PythonTab() {
  return (
    <section className="space-y-4">
      <p>Install from PyPI:</p>
      <CodeBlock>pip install jsoncompat</CodeBlock>
      <CodeBlock>{`import jsoncompat as jsc

old_schema = '{"type": "string"}'
new_schema = '{"type": "number"}'

print(jsc.check_compat(old_schema, new_schema, role="both"))`}</CodeBlock>
      <p>
        See more usage examples{" "}
        <a
          className="text-blue-600 hover:underline"
          href="https://github.com/ostrowr/jsoncompat/tree/main/examples/python"
          target="_blank"
          rel="noopener"
        >
          here.
        </a>
        .
      </p>
    </section>
  );
}

function JsTab() {
  return (
    <section className="space-y-4">
      <p>Browser / Node via WebAssembly:</p>
      <CodeBlock>npm i jsoncompat</CodeBlock>
      <CodeBlock>{`import init, { check_compat } from "jsoncompat";

await init(); // or init(wasmUrl) with Vite bundlers

const ok = await check_compat('{"type":"string"}', '{"type":"number"}', "both");`}</CodeBlock>
      <p>
        See more usage examples{" "}
        <a
          className="text-blue-600 hover:underline"
          href="https://github.com/ostrowr/jsoncompat/tree/main/examples/wasm"
          target="_blank"
          rel="noopener"
        >
          here
        </a>
        .
      </p>
    </section>
  );
}

function Links() {
  return (
    <section>
      <h2 className="mb-2 text-xl font-semibold">Links</h2>
      <ul className="list-inside list-disc space-y-1">
        <li>
          <a
            className="text-blue-600 hover:underline"
            href="https://github.com/ostrowr/jsoncompat"
          >
            GitHub repository
          </a>
        </li>
        <li>
          <a
            className="text-blue-600 hover:underline"
            href="https://crates.io/crates/jsoncompat"
          >
            crates.io
          </a>
        </li>
        <li>
          <a
            className="text-blue-600 hover:underline"
            href="https://pypi.org/project/jsoncompat/"
          >
            PyPI
          </a>
        </li>
        <li>
          <a
            className="text-blue-600 hover:underline"
            href="https://www.npmjs.com/package/jsoncompat"
          >
            npm
          </a>
        </li>
      </ul>
    </section>
  );
}
