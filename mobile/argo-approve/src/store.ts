import { Store } from "@tauri-apps/plugin-store";
import { SETTINGS_DEFAULTS, type AppSettings } from "./types";

const STORE_FILE = "settings.json";
const KEY = "settings";

let cached: Store | null = null;

async function open(): Promise<Store> {
  if (!cached) {
    cached = await Store.load(STORE_FILE);
  }
  return cached;
}

export async function loadSettings(): Promise<AppSettings> {
  try {
    const store = await open();
    const value = (await store.get(KEY)) as Partial<AppSettings> | null;
    if (!value) return { ...SETTINGS_DEFAULTS };
    return {
      gateway_url: value.gateway_url ?? SETTINGS_DEFAULTS.gateway_url,
      bearer_token: value.bearer_token ?? SETTINGS_DEFAULTS.bearer_token,
    };
  } catch {
    return { ...SETTINGS_DEFAULTS };
  }
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  const store = await open();
  await store.set(KEY, settings);
  await store.save();
}
