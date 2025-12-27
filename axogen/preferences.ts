import { existsSync } from "fs";
import { mkdir, readFile, writeFile } from "fs/promises";
import { join } from "path";

export interface Preferences {
    nodePackageManager?: "bun" | "npm";
    pythonPackageManager?: "venv" | "poetry";
}

const PREFERENCES_DIR = ".axogen";
const PREFERENCES_FILE = join(PREFERENCES_DIR, "preferences.json");

export async function loadPreferences(): Promise<Preferences> {
    if (!existsSync(PREFERENCES_FILE)) {
        return {};
    }

    try {
        const content = await readFile(PREFERENCES_FILE, "utf-8");
        return JSON.parse(content) as Preferences;
    } catch (error) {
        return {};
    }
}

export async function savePreferences(preferences: Preferences): Promise<void> {
    if (!existsSync(PREFERENCES_DIR)) {
        await mkdir(PREFERENCES_DIR, { recursive: true });
    }

    await writeFile(PREFERENCES_FILE, JSON.stringify(preferences, null, 2), "utf-8");
}

export async function getPreference<K extends keyof Preferences>(
    key: K
): Promise<Preferences[K] | undefined> {
    const prefs = await loadPreferences();
    return prefs[key];
}

export async function setPreference<K extends keyof Preferences>(
    key: K,
    value: Preferences[K]
): Promise<void> {
    const prefs = await loadPreferences();
    prefs[key] = value;
    await savePreferences(prefs);
}
