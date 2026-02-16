export type ControlAction = "next" | "prev" | "reset" | "pause" | "toggle_debug";

export class KeyController {
  private readonly handler: (event: KeyboardEvent) => void;

  public constructor(onAction: (action: ControlAction) => void) {
    this.handler = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if (key === "arrowright" || key === "n") {
        onAction("next");
        event.preventDefault();
        return;
      }
      if (key === "arrowleft" || key === "p") {
        onAction("prev");
        event.preventDefault();
        return;
      }
      if (key === "r") {
        onAction("reset");
        event.preventDefault();
        return;
      }
      if (key === " ") {
        onAction("pause");
        event.preventDefault();
        return;
      }
      if (key === "d") {
        onAction("toggle_debug");
        event.preventDefault();
      }
    };

    window.addEventListener("keydown", this.handler);
  }

  public dispose(): void {
    window.removeEventListener("keydown", this.handler);
  }
}
