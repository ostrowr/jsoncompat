import { createFileRoute } from "@tanstack/react-router";
import type { ReactNode } from "react";

export const Route = createFileRoute("/usage")({
  component: RouteComponent,
});

function UsagePage(): ReactNode {
  return (
    <main className="mx-auto max-w-4xl px-4 py-8 space-y-8">
      <h1 className="text-3xl font-bold mb-4">Usage</h1>

      <section className="space-y-4">
        <h2 id="ci" className="text-xl font-semibold">
          <a href="#ci">Continuous Integration (CI)</a>
        </h2>
        <p>
          Use <code>jsoncompat ci</code> in your CI pipelines to ensure that
          changes to your JSON schemas are compatible.{" "}
          <code>jsoncompat ci</code> takes a file containing a mapping from
          schema IDs to schemas and modes, and pretty-prints compatibility
          results between all of them, along with incompatible examples if it
          can find them. Here's an{" "}
          <a
            href="https://github.com/ostrowr/jsoncompat/pull/31"
            className="text-blue-600 hover:underline"
          >
            example of a CI pipeline
          </a>{" "}
          failing due to a backwards-incompatible change.
        </p>

        <p>
          An example GitHub actions workflow that checks compatibility between a
          pull request and its merge base can be found{" "}
          <a
            href="https://github.com/ostrowr/jsoncompat/blob/main/.github/workflows/compat.yml"
            className="text-blue-600 hover:underline"
          >
            here
          </a>
          .
        </p>
        <figure className="my-6">
          <img
            src="/ci_results.png"
            alt="Screenshot of jsoncompat CI results"
            className="rounded border border-gray-200 shadow-sm mx-auto"
            style={{ maxWidth: "100%", height: "auto" }}
          />
          <figcaption className="text-center text-sm text-gray-500 mt-2">
            Example output of <code>jsoncompat ci</code> in a CI pipeline
          </figcaption>
        </figure>

        <p>
          Run <code>jsoncompat ci --help</code> for more information.
        </p>
      </section>

      <section className="space-y-4">
        <h2 id="compat" className="text-xl font-semibold">
          <a href="#compat">Check compatibility between two JSON schemas</a>
        </h2>
        <p>
          <a href="/checker" className="text-blue-600 hover:underline">
            Try it out in your browser &rarr;
          </a>
        </p>
        <p>
          Use <code>jsoncompat compat</code> to check compatibility between two
          json schemas. By default, <code>jsoncompat compat</code> only does
          static analysis. Since this static analysis is not complete, you can
          also specify a <code>--fuzz</code> parameter to generate random
          examples and check them for compatibility.
        </p>
        <figure className="my-6">
          <img
            src="/compat.png"
            alt="Screenshot of jsoncompat compat results"
            className="rounded border border-gray-200 shadow-sm mx-auto"
            style={{ maxWidth: "100%", height: "auto" }}
          />
          <figcaption className="text-center text-sm text-gray-500 mt-2">
            Example output of <code>jsoncompat compat</code> between two schemas
          </figcaption>
        </figure>
        <p>
          Run <code>jsoncompat compat --help</code> for more information.
        </p>
      </section>

      <section className="space-y-4">
        <h2 id="generate" className="text-xl font-semibold">
          <a href="#generate">Generate representative examples</a>
        </h2>
        <p>
          <a href="/fuzzer" className="text-blue-600 hover:underline">
            Try it out in your browser &rarr;
          </a>
        </p>
        <p>
          You can use <code>jsoncompat generate</code> to generate examples that
          fulfill some json schema.
        </p>
        <figure className="my-6">
          <img
            src="/generate.png"
            alt="Screenshot of jsoncompat generate results"
            className="rounded border border-gray-200 shadow-sm mx-auto"
            style={{ maxWidth: "100%", height: "auto" }}
          />
          <figcaption className="text-center text-sm text-gray-500 mt-2">
            Example output of <code>jsoncompat generate</code>
          </figcaption>
        </figure>
        <p>
          Run <code>jsoncompat generate --help</code> for more information.
        </p>
      </section>
    </main>
  );
}

// Replace the default export with the new UsagePage
function RouteComponent(): ReactNode {
  return <UsagePage />;
}
