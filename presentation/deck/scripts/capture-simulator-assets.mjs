import { mkdir, mkdtemp, readdir, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import process from "node:process";
import { chromium } from "playwright";

const deckRoot = path.resolve(import.meta.dirname, "..");
const artifactsRoot = path.join(deckRoot, "artifacts");
const stillsRoot = path.join(artifactsRoot, "simulator-stills");
const gifsRoot = path.join(artifactsRoot, "simulator-gifs");
const devPort = 3030;
const baseUrl = `http://localhost:${devPort}`;

const demoSelectorFor = (title) => `[data-demo-title="${title}"]`;

const beats = [
  {
    name: "required-field-failure",
    title: "Beat 1: add a required field",
    action: async (page) => {
      await page.locator(demoSelectorFor("Beat 1: add a required field")).last().getByTestId("run-beat").click();
      await page.waitForTimeout(3800);
    },
  },
  {
    name: "shape-change-failure",
    title: "Beat 2: change the shape",
    action: async (page) => {
      await page.locator(demoSelectorFor("Beat 2: change the shape")).last().getByTestId("run-beat").click();
      await page.waitForTimeout(4200);
    },
  },
  {
    name: "reader-union-bridge",
    title: "Beat 3: bridge with a reader union",
    action: async (page) => {
      await page.locator(demoSelectorFor("Beat 3: bridge with a reader union")).last().getByTestId("run-beat").click();
      await page.waitForTimeout(4200);
    },
  },
];

const sleep = (ms) => new Promise((resolve) => {
  setTimeout(resolve, ms);
});

const runCommand = async (command, args, options = {}) => {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      stdio: "inherit",
      cwd: deckRoot,
      env: process.env,
      ...options,
    });
    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(" ")} exited with code ${code}`));
    });
    child.on("error", reject);
  });
};

const waitForServer = async () => {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(baseUrl);
      if (response.ok) {
        return;
      }
    } catch {
      // Server not ready yet.
    }
    await sleep(1000);
  }
  throw new Error(`deck dev server did not become ready at ${baseUrl}`);
};

const startDeckServer = () => {
  const child = spawn("python3", ["-m", "http.server", String(devPort), "-d", "dist"], {
    cwd: deckRoot,
    stdio: "inherit",
    env: process.env,
  });
  return child;
};

const gotoSlideByTitle = async (page, title) => {
  await page.goto(baseUrl, { waitUntil: "networkidle" });
  for (let index = 0; index < 28; index += 1) {
    const heading = page.getByRole("heading", { name: title });
    if (await heading.count() > 0 && await heading.first().isVisible()) {
      return;
    }
    await page.keyboard.press("ArrowRight");
    await page.waitForTimeout(220);
  }
  throw new Error(`unable to find slide titled '${title}'`);
};

const captureGif = async (page, gifPath) => {
  const frameDir = await mkdtemp(path.join(os.tmpdir(), "jsoncompat-demo-frames-"));
  try {
    for (let frameIndex = 0; frameIndex < 18; frameIndex += 1) {
      const framePath = path.join(frameDir, `frame-${String(frameIndex).padStart(3, "0")}.png`);
      await page.screenshot({ path: framePath });
      await page.waitForTimeout(180);
    }
    const ffmpeg = spawn(
      "ffmpeg",
      [
        "-y",
        "-framerate",
        "6",
        "-i",
        path.join(frameDir, "frame-%03d.png"),
        "-vf",
        "fps=12,scale=1600:-1:flags=lanczos",
        gifPath,
      ],
      { stdio: "inherit" },
    );
    await new Promise((resolve, reject) => {
      ffmpeg.on("exit", (code) => {
        if (code === 0) {
          resolve();
          return;
        }
        reject(new Error(`ffmpeg exited with code ${code}`));
      });
      ffmpeg.on("error", reject);
    });
  } finally {
    await rm(frameDir, { recursive: true, force: true });
  }
};

await mkdir(stillsRoot, { recursive: true });
await mkdir(gifsRoot, { recursive: true });

let server = null;

try {
  await runCommand("npm", ["run", "build"]);
  server = startDeckServer();
  await waitForServer();
  const browser = await chromium.launch({
    headless: true,
    args: ["--use-gl=swiftshader", "--enable-unsafe-swiftshader"],
  });
  const page = await browser.newPage({
    viewport: {
      width: 1600,
      height: 900,
    },
  });

  for (const beat of beats) {
    await gotoSlideByTitle(page, beat.title);
    await beat.action(page);
    await page.screenshot({
      path: path.join(stillsRoot, `${beat.name}.png`),
    });

    await gotoSlideByTitle(page, beat.title);
    await beat.action(page);
    await captureGif(page, path.join(gifsRoot, `${beat.name}.gif`));
  }

  await browser.close();
} finally {
  server?.kill("SIGTERM");
  const currentArtifacts = await readdir(artifactsRoot, { withFileTypes: true });
  if (currentArtifacts.length === 0) {
    await rm(artifactsRoot, { recursive: true, force: true });
  }
}
