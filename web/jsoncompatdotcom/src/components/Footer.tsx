export default function Footer() {
  return (
    <footer
      className="fixed bottom-0 right-0 m-4 text-sm text-gray-500 bg-white/80 rounded px-3 py-1 shadow"
      style={{ zIndex: 50 }}
    >
      find more projects at{" "}
      <a
        href="https://ostro.ws"
        target="_blank"
        rel="noopener"
        className="text-blue-600 hover:underline"
      >
        ostro.ws
      </a>
    </footer>
  );
}
