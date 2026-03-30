import { spawn } from "node:child_process";
import { rm } from "node:fs/promises";
import { resolve } from "node:path";

const websiteRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(websiteRoot, "..", "..");
const deckRoot = resolve(repoRoot, "presentation", "deck");
const deckOutput = resolve(websiteRoot, "public", "deck");
const slidevBin = resolve(deckRoot, "node_modules", ".bin", "slidev");

async function run(command, args, cwd) {
	const child = spawn(command, args, {
		cwd,
		stdio: "inherit",
	});

	const exitCode = await new Promise((resolveExitCode, reject) => {
		child.on("error", reject);
		child.on("exit", resolveExitCode);
	});

	if (exitCode !== 0) {
		throw new Error(
			`${command} ${args.join(" ")} failed with exit code ${exitCode}`,
		);
	}
}

await rm(deckOutput, { force: true, recursive: true });
await run(
	slidevBin,
	["build", "--base", "/deck/", "--out", deckOutput],
	deckRoot,
);
