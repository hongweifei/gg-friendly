import { existsSync, readdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

/**
 * Playwright looks for an exact browser revision, which may not be the one
 * installed on this machine. Fall back to whichever chromium is present.
 */
export function installedChromium(): string | undefined {
    let root = join(homedir(), ".cache", "ms-playwright");
    if (!existsSync(root)) return undefined;

    let candidates = readdirSync(root)
        .filter((entry) => entry.startsWith("chromium_headless_shell-"))
        .map((entry) => join(root, entry, "chrome-headless-shell-linux64", "chrome-headless-shell"));

    return candidates.sort().reverse().find((path) => existsSync(path));
}
