import { spawn } from "node:child_process";
import { rm } from "node:fs/promises";
import { resolve } from "node:path";

const websiteRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(websiteRoot, "..", "..");
const deckRoot = resolve(repoRoot, "presentation", "deck");
const deckOutput = resolve(websiteRoot, "public", "deck");

await rm(deckOutput, { force: true, recursive: true });

const child = spawn(
	"npm",
	["exec", "--", "slidev", "build", "--base", "/deck/", "--out", deckOutput],
	{
		cwd: deckRoot,
		stdio: "inherit",
	},
);

const exitCode = await new Promise((resolveExitCode, reject) => {
	child.on("error", reject);
	child.on("exit", resolveExitCode);
});

if (exitCode !== 0) {
	throw new Error(`deck build failed with exit code ${exitCode}`);
}
