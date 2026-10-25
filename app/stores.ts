import type { MutationResult } from "./messages/MutationResult";
import type { ProgressEvent } from "./messages/ProgressEvent";
import type { RepoConfig } from "./messages/RepoConfig";
import type { RepoStatus } from "./messages/RepoStatus";
import type { RevHeader } from "./messages/RevHeader";
import type { RevSet } from "./messages/RevSet";
import type { Operand } from "./messages/Operand";
import { writable } from "svelte/store";
import { event, type Query } from "./events";
import type { InputRequest } from "./messages/InputRequest";
import type { InputResponse } from "./messages/InputResponse";
import type { RevChange } from "./messages/RevChange";

export const repoConfigEvent = await event<RepoConfig>("gg://repo/config", { type: "Initial" });
export const repoStatusEvent = await event<RepoStatus | undefined>("gg://repo/status", undefined);
export const revisionSelectEvent = await event<RevSet | undefined>("gg://revision/select", undefined);
export const changeSelectEvent = await event<RevChange | undefined>("gg://change/select", undefined);
export const progressEvent = await event<ProgressEvent | undefined>("gg://progress", undefined);

export const currentMutation = writable<Query<MutationResult> | null>(null);
export const currentContext = writable<Operand | null>();
export const currentSource = writable<Operand | null>();
export const currentTarget = writable<Operand | null>();
export const currentInput = writable<InputRequest & { callback: (response: InputResponse | null) => void; } | null>();

export const hasModal = writable<boolean>(false);
export const hasMenu = writable<{ x: number; y: number; } | null>(null);
export const logQueryRequest = writable<string | null>(null);
export const lastFocus = writable<number>(Date.now());
export const ignoreToggled = writable<boolean>(false);

/**
 * Counts how many times the repository has been changed by an operation.
 *
 * The status event says what the repository looks like now, but it only
 * arrives when the operation moved the working copy. Copying a change or
 * renaming a branch leaves the working copy alone, so the status looks the
 * same afterwards even though the repository is different. This counter is
 * the signal that something happened at all.
 */
export const repoChanges = writable<number>(0);

/**
 * The text the user is searching the change list for. The top bar and the
 * list's own search box share this, so there is only ever one search.
 */
export const searchText = writable<string>("");
export const searchOpen = writable<boolean>(false);

/**
 * Which theme to use. "system" follows the operating system's setting, and is
 * the default. The repository's own setting still applies when the user has
 * not picked one here.
 */
export type ThemeChoice = "system" | "light" | "dark";
export const themeChoice = writable<ThemeChoice>("system");

/**
 * The branch the user picked in the branch list. The change list is narrowed
 * to it, and it is kept apart from the query dropdown so picking a branch
 * does not change what the dropdown shows.
 */
export const branchFilter = writable<string | null>(null);

export const selectionHeaders = writable<RevHeader[]>([]);

export const descriptionDraft = writable<{ changeKey: string; text: string; } | null>(null);

export function dragOverWidget(event: DragEvent) {
    event.stopPropagation();
    currentTarget.set(null);
}