import type { SessionCommand, View } from "./types";
import { mockBackend } from "./mock/backend";

export const inTauri = "__TAURI_INTERNALS__" in window;

export interface Backend {
  getView(): Promise<View>;
  command(command: SessionCommand): Promise<View>;
}

const tauriBackend: Backend = {
  async getView() {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<View>("get_view");
  },
  async command(command) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<View>("session_command", { command });
  },
};

/** The real engine under Tauri; a scripted stand-in in a plain browser. */
export const backend: Backend = inTauri ? tauriBackend : mockBackend;

export async function closeWindow(): Promise<void> {
  if (!inTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}

export async function fitWindowHeight(width: number, height: number): Promise<void> {
  if (!inTauri) return;
  const [{ getCurrentWindow }, { LogicalSize }] = await Promise.all([
    import("@tauri-apps/api/window"),
    import("@tauri-apps/api/dpi"),
  ]);
  await getCurrentWindow().setSize(new LogicalSize(width, Math.ceil(height)));
}
