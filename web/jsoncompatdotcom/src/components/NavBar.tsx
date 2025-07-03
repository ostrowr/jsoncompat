import { Link, useRouterState } from "@tanstack/react-router";

// Responsive navigation bar with active link highlighting
export default function NavBar() {
  const { location } = useRouterState();
  const current = location.pathname;

  const linkClasses = (href: string) =>
    `px-3 py-2 rounded-md text-sm font-medium ${
      current === href
        ? "bg-blue-700 text-white"
        : "text-gray-300 hover:bg-blue-500 hover:text-white"
    }`;

  return (
    <nav className="bg-blue-600 sticky top-0 z-10">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="flex h-16 items-center justify-between">
          <div className="flex items-center space-x-4">
            <Link to="/" className="text-xl font-semibold text-white">
              jsoncompat
            </Link>

            <div className="hidden md:flex md:items-baseline md:space-x-2">
              <Link to="/checker" className={linkClasses("/checker")}>
                Checker
              </Link>
              <Link to="/fuzzer" className={linkClasses("/fuzzer")}>
                Fuzzer
              </Link>
              <Link to="/install" className={linkClasses("/install")}>
                Install
              </Link>
              <Link to="/usage" className={linkClasses("/usage")}>
                Usage
              </Link>
            </div>
          </div>

          <a
            href="https://github.com/ostrowr/jsoncompat"
            target="_blank"
            rel="noopener noreferrer"
            className="text-gray-300 hover:text-white"
          >
            <svg className="h-6 w-6" fill="currentColor" viewBox="0 0 24 24">
              <title>GitHub</title>
              <path
                fillRule="evenodd"
                d="M12 2C6.477 2 2 6.485 2 12.012c0 4.417 2.865 8.166 6.839 9.489.5.092.682-.217.682-.483 0-.237-.009-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.155-1.11-1.462-1.11-1.462-.908-.622.069-.609.069-.609 1.004.071 1.533 1.032 1.533 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.339-2.221-.253-4.555-1.112-4.555-4.943 0-1.092.39-1.987 1.03-2.688-.103-.254-.446-1.271.098-2.65 0 0 .84-.27 2.75 1.028a9.564 9.564 0 0 1 2.5-.338c.85.004 1.705.115 2.5.338 1.909-1.298 2.748-1.028 2.748-1.028.546 1.379.203 2.396.1 2.65.642.701 1.029 1.596 1.029 2.688 0 3.842-2.337 4.687-4.565 4.936.358.309.678.918.678 1.852 0 1.336-.012 2.414-.012 2.744 0 .268.18.58.688.482A10.013 10.013 0 0 0 22 12.012C22 6.485 17.523 2 12 2Z"
                clipRule="evenodd"
              />
            </svg>
          </a>
        </div>
      </div>
    </nav>
  );
}
