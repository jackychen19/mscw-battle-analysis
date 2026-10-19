import type { Image, Permission, RectPt, Regions, Missing, Sample, SessionCommand, SetupStatus, View, WindowInfo } from "./types";
import { mockBackend, mockSetup } from "./mock/backend";

export const inTauri = "__TAURI_INTERNALS__" in window;

export interface Backend {
  getView(): Promise<View>;
  command(command: SessionCommand): Promise<View>;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const core = await import("@tauri-apps/api/core");
  return core.invoke<T>(cmd, args);
}

const tauriBackend: Backend = {
  getView: () => invoke<View>("get_view"),
  command: (command) => invoke<View>("session_command", { command }),
};

/** The real engine under Tauri; a scripted stand-in in a plain browser. */
export const backend: Backend = inTauri ? tauriBackend : mockBackend;

/** Calibration window commands. */
export interface Setup {
  status(): Promise<SetupStatus>;
  requestPermission(): Promise<Permission>;
  openPermissionSettings(): Promise<void>;
  chooseWindow(id: number): Promise<WindowInfo>;
  windowPreview(): Promise<Image>;
  regionPreview(region: RectPt): Promise<Image>;
  teach(regions: Regions, sample: Sample): Promise<Missing>;
  reset(): Promise<void>;
}

function decodeImage(buf: ArrayBuffer): Image {
  const head = new DataView(buf, 0, 8);
  const width = head.getUint32(0, true);
  const height = head.getUint32(4, true);
  return { width, height, rgba: new Uint8ClampedArray(buf, 8, width * height * 4) };
}

const tauriSetup: Setup = {
  status: () => invoke("setup_status"),
  requestPermission: () => invoke("request_permission"),
  openPermissionSettings: () => invoke("open_permission_settings"),
  chooseWindow: (id) => invoke("choose_window", { id }),
  windowPreview: async () => decodeImage(await invoke<ArrayBuffer>("window_preview")),
  regionPreview: async (region) => decodeImage(await invoke<ArrayBuffer>("region_preview", { region })),
  teach: (regions, sample) => invoke("teach_sample", { regions, sample }),
  reset: () => invoke("reset_calibration"),
};

export const setup: Setup = inTauri ? tauriSetup : mockSetup;

/** Label of the current window (`main` or `calibrate`); `?window=` in a plain browser. */
export async function windowLabel(): Promise<string> {
  if (!inTauri) return new URLSearchParams(window.location.search).get("window") ?? "main";
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().label;
}

export async function openCalibration(): Promise<void> {
  if (!inTauri) {
    window.open("?window=calibrate", "_blank");
    return;
  }
  await invoke("open_calibration");
}

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
