import { startInteractiveApp } from "./app";

const mountPoint = document.getElementById("app");
if (mountPoint === null) {
  throw new Error("missing #app mount point");
}

void startInteractiveApp(mountPoint);
