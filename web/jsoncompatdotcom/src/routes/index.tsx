import { Link, createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <main className="bg-gray-50">
      {/* Hero */}
      <section className="mx-auto max-w-7xl px-4 py-24 text-center">
        <h1 className="text-5xl font-extrabold tracking-tight text-gray-900 sm:text-6xl">
          jsoncompat
        </h1>
        <p className="mx-auto mt-6 max-w-3xl text-lg text-gray-700">
          Safely evolve your JSON schemas. Check backward/forward compatibility
          and automatically generate representative sample data. Available in
          Rust, Python and WebAssembly.
        </p>

        <div className="mt-8 flex justify-center gap-4">
          <Link
            to="/checker"
            className="rounded bg-blue-600 px-6 py-3 font-medium text-white hover:bg-blue-700"
          >
            Try the checker
          </Link>
          <Link
            to="/fuzzer"
            className="rounded bg-white px-6 py-3 font-medium text-blue-600 shadow ring-1 ring-inset ring-blue-600 hover:bg-blue-50"
          >
            Generate values
          </Link>
        </div>
      </section>

      {/* Features */}
      <section className="bg-white py-16">
        <div className="mx-auto grid max-w-5xl grid-cols-1 gap-12 px-4 md:grid-cols-3">
          <Feature title="Escape version skew">
            Don't want to deal with protobuf? Keep using JSON, but safely.
          </Feature>
          <Feature title="Multi-platform (and fast)">
            Same engine for Rust, Python, & the browser.
          </Feature>
          <Feature title="Open source">
            jsoncompat is open source and free to use.
          </Feature>
        </div>
      </section>
    </main>
  );
}

function Feature({ title, children }: { title: string; children: string }) {
  return (
    <div className="text-center">
      <h3 className="mb-2 text-lg font-semibold text-gray-900">{title}</h3>
      <p className="text-gray-600">{children}</p>
    </div>
  );
}
