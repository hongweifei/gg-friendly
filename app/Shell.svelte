<script lang="ts">
    import type { RevSet } from "./messages/RevSet";
    import type { RevsResult } from "./messages/RevsResult";
    import type { RepoConfig } from "./messages/RepoConfig";
    import type { RepoStatus } from "./messages/RepoStatus";
    import { type Query, query, mutate, trigger, onEvent, isTauri, getInput } from "./ipc.js";
    import {
        currentMutation,
        currentContext,
        repoConfigEvent,
        repoStatusEvent,
        repoChanges,
        revisionSelectEvent,
        selectionHeaders,
        currentInput,
        hasMenu,
        progressEvent,
        lastFocus,
        ignoreToggled,
    } from "./stores.js";
    import ContextMenu from "./controls/ContextMenu.svelte";
    import RefMutator from "./mutators/RefMutator";
    import ChangeMutator from "./mutators/ChangeMutator";
    import RevisionMutator from "./mutators/RevisionMutator";
    import WorkspaceMutator from "./mutators/WorkspaceMutator";
    import Pane from "./shell/Pane.svelte";
    import TitlebarInset from "./shell/TitlebarInset.svelte";
    import Zone from "./objects/Zone.svelte";
    import StatusBar from "./shell/StatusBar.svelte";
    import ModalOverlay from "./shell/ModalOverlay.svelte";
    import ErrorDialog from "./shell/ErrorDialog.svelte";
    import ProgressDialog from "./shell/ProgressDialog.svelte";
    import RecentWorkspaces from "./shell/RecentWorkspaces.svelte";
    import { onMount, setContext } from "svelte";
    import { themeChoice } from "./stores.js";
    import InputDialog from "./shell/InputDialog.svelte";
    import type Settings from "./shell/Settings";
    import type { RepoEvent } from "./messages/RepoEvent";
    import RepositoryMutator from "./mutators/RepositoryMutator";

    interface $$Slots {
        default: {
            workspace: Extract<RepoConfig, { type: "Workspace" }>;
            selection: Query<RevsResult>;
            selectedBranchNames: Set<string>;
        };
        banner: {};
    }

    /** the branches the selected change is on, for the branch list */
    let selectedBranchNames: Set<string> = new Set();

    export let revsetOverride: string | null = null;

    let selection: Query<RevsResult> = {
        type: "wait",
    };
    // for open recent workspaces when error dialogs happen
    let recentWorkspaces: string[] = [];

    if (isTauri()) {
        document.addEventListener("keydown", (event) => {
            const key = event.key.toLowerCase();
            // webviews don't pass these to the native menu, and webview2 would reload on f5
            if (((key === "n" || key === "m" || key === "o") && event.ctrlKey) || key === "f5") {
                event.preventDefault();
                trigger("forward_accelerator", {
                    key,
                    ctrl: event.ctrlKey,
                    shift: event.shiftKey,
                });
            }
        });
    }

    // The context is cleared when a click lands outside the context menu.
    // The listener runs in the bubble phase, so a click on a menu entry
    // reaches the entry's own handler first.
    document.body.addEventListener("click", () => currentContext.set(null));

    async function initialize() {
        const result = await query<RepoConfig>("query_workspace", null);
        if ($repoConfigEvent.type === "TimeoutError") {
            return; // too late! we're dead!
        } else if (result.type === "data") {
            repoConfigEvent.set(result.value);
        } else {
            repoConfigEvent.set({
                type: "LoadError",
                absolute_path: "",
                message: result.message,
            });
        }
    }
    initialize();

    let loadTimeout: number | null;
    onMount(() => {
        if ($repoConfigEvent.type == "Initial") {
            loadTimeout = setTimeout(() => {
                repoConfigEvent.set({ type: "TimeoutError" });
            }, 10_000);
        }

        if (!isTauri()) {
            // signal shutdown on unload, cancel if it was a reload
            const handleUnload = () => {
                trigger("begin_shutdown");
            };
            trigger("end_shutdown");
            window.addEventListener("beforeunload", handleUnload);

            // snapshot when focusing the browser or returning to the tab
            document.addEventListener("visibilitychange", handleFocus);
            window.addEventListener("focus", handleFocus);

            // ping the backend; if either component dies, the other can clean up
            const heartbeatInterval = setInterval(async () => {
                trigger("heartbeat", undefined, cleanup);
            }, 30_000);

            const cleanup = () => {
                clearInterval(heartbeatInterval);
                document.removeEventListener("visibilitychange", handleFocus);
                window.removeEventListener("focus", handleFocus);
                window.removeEventListener("beforeunload", handleUnload);
            };
            return cleanup;
        }
    });

    let settings: Settings = {
        markUnpushedBookmarks: true,
    };
    setContext<Settings>("settings", settings);

    if (isTauri()) {
        // web mode: mutations done directly by ContextMenu
        // gui mode: mutations triggered by native context menu
        onEvent("gg://context/revision", mutateRevision);
        onEvent("gg://context/tree", mutateTree);
        onEvent("gg://context/bookmark", mutateRef);
        onEvent("gg://context/workspace", mutateWorkspace);
        onEvent("gg://menu/repo", mutateRepository);
        // gui mode: snapshot when window gains focus
        onEvent("gg://focus", handleFocus);
    }

    $: if ($repoConfigEvent) loadRepo($repoConfigEvent);
    $: if ($repoStatusEvent && revsetOverride && !$revisionSelectEvent) {
        let syntheticId = {
            change: {
                type: "ChangeId" as const,
                hex: revsetOverride,
                prefix: revsetOverride,
                rest: "",
                offset: null,
                is_divergent: false,
            },
            commit: { type: "CommitId" as const, hex: "", prefix: "", rest: "" },
        };
        $revisionSelectEvent = { from: syntheticId, to: syntheticId };
    }
    $: if ($repoStatusEvent && $revisionSelectEvent) {
        loadChange($revisionSelectEvent);
    }
    // gui mode: the backend needs the selection for native menu enablement
    $: if (isTauri()) trigger("set_selection", { set: $revisionSelectEvent ?? null });
    $: if (!isTauri()) {
        document.title =
            $repoConfigEvent.type === "Workspace"
                ? "GG - " + $repoConfigEvent.absolute_path.split("/").pop()
                : "GG - Gui for JJ";
    }
    $: if (
        $repoConfigEvent.type === "LoadError" ||
        $repoConfigEvent.type === "TimeoutError" ||
        $repoConfigEvent.type === "WorkerError"
    ) {
        queryRecentWorkspaces();
    }

    async function loadRepo(config: RepoConfig) {
        if (loadTimeout) {
            clearTimeout(loadTimeout);
            loadTimeout = null;
        }

        $revisionSelectEvent = undefined;
        if (config.type == "Workspace") {
            settings.markUnpushedBookmarks = config.mark_unpushed_bookmarks;
            ignoreToggled.set(config.ignore_immutable);
            $repoStatusEvent = config.status;
        }
    }

    async function loadChange(set: RevSet) {
        let rev = await query<RevsResult>("query_revisions", { set }, (q) => (selection = q));
        selectedBranchNames = new Set();

        // if empty, fall back to working copy
        if (
            rev.type == "data" &&
            rev.value.type == "NotFound" &&
            (set.from.commit.hex !== $repoStatusEvent?.working_copy?.hex ||
                set.to.commit.hex !== $repoStatusEvent?.working_copy?.hex)
        ) {
            const workingCopyId = {
                change: {
                    type: "ChangeId" as const,
                    hex: "@",
                    prefix: "@",
                    rest: "",
                    offset: null,
                    is_divergent: false,
                },
                commit: $repoStatusEvent!.working_copy,
            };
            return loadChange({ from: workingCopyId, to: workingCopyId });
        }

        selection = rev;
        if (rev.type == "data" && rev.value.type == "Detail") {
            $selectionHeaders = rev.value.headers;

            let names = new Set<string>();
            for (let header of rev.value.headers) {
                for (let ref of header.refs) {
                    if (ref.type == "LocalBookmark") {
                        names.add(ref.bookmark_name);
                    }
                }
            }
            selectedBranchNames = names;
        }
    }

    async function handleFocus() {
        if (!isTauri() && document.visibilityState !== "visible") {
            return;
        }
        if ($repoConfigEvent.type === "Workspace") {
            let result = await query<RepoStatus | null>("query_snapshot", null);
            if (result.type === "data" && result.value) {
                repoStatusEvent.set(result.value);
                // Taking the snapshot can pick up file changes made outside the
                // app, so the lists have to be read again.
                repoChanges.update((n) => n + 1);
            }
        }
        lastFocus.set(Date.now());
    }

    async function queryRecentWorkspaces() {
        const result = await query<string[]>("query_recent_workspaces", null);
        recentWorkspaces = result.type === "data" ? result.value : [];
    }

    function mutateRevision(event: string) {
        if ($currentContext?.type == "Revision") {
            new RevisionMutator([$currentContext.header], $ignoreToggled).handle(event);
        } else if ($currentContext?.type == "Revisions") {
            new RevisionMutator($currentContext.headers, $ignoreToggled).handle(event);
        }
        $currentContext = null;
    }

    function mutateTree(event: string) {
        if ($currentContext?.type == "Change") {
            new ChangeMutator(
                $currentContext.headers,
                $currentContext.path,
                $currentContext.hunk,
                $ignoreToggled,
            ).handle(event);
        }
        $currentContext = null;
    }

    function mutateRef(event: string) {
        if ($currentContext?.type == "Ref") {
            new RefMutator($currentContext.ref, $ignoreToggled).handle(event);
        }
        $currentContext = null;
    }

    function mutateWorkspace(event: string) {
        if ($currentContext?.type == "Workspace") {
            new WorkspaceMutator($currentContext.name).handle(event);
        }
        $currentContext = null;
    }

    function mutateRepository(event: RepoEvent) {
        new RepositoryMutator().handle(event);
    }

    // the user's pick wins; without one the repository's setting applies,
    // and with neither the system setting is followed
    $: shellTheme = (() => {
        if ($themeChoice != "system") {
            return $themeChoice;
        }
        let override = $repoConfigEvent?.type == "Workspace" ? $repoConfigEvent.theme_override : null;
        return override ?? "";
    })();

    // remember the pick across restarts
    onMount(() => {
        let saved = localStorage.getItem("gg.theme");
        if (saved == "light" || saved == "dark" || saved == "system") {
            themeChoice.set(saved);
        }

        const unsubscribe = themeChoice.subscribe((choice) => {
            localStorage.setItem("gg.theme", choice);
        });
        return unsubscribe;
    });
</script>

<Zone operand={{ type: "Repository" }} alwaysTarget let:target>
    <div id="shell" class={shellTheme}>
        {#if $repoConfigEvent.type == "Initial"}
            <div class="content">
                <TitlebarInset>
                    <Pane>
                        <h2 slot="header">正在加载...</h2>
                    </Pane>
                </TitlebarInset>
            </div>
        {:else if $repoConfigEvent.type == "Workspace"}
            <div class="banner">
                <slot name="banner" />
            </div>
            <div class="content">
                <slot workspace={$repoConfigEvent} {selection} {selectedBranchNames} />
            </div>
        {:else if $repoConfigEvent.type == "LoadError"}
            <ModalOverlay>
                <ErrorDialog title="没有打开仓库">
                    <p style="grid-column: 1/3">
                        请在 Jujutsu 仓库目录里运行 <code>gg</code>，或者从「仓库」菜单里打开一个。
                    </p>
                    <RecentWorkspaces workspaces={recentWorkspaces} />
                </ErrorDialog>
            </ModalOverlay>
        {:else if $repoConfigEvent.type == "TimeoutError"}
            <ModalOverlay>
                <ErrorDialog title="没有打开仓库" severe>
                    <p>和后台的连接超时了，这次操作花的时间太长。</p>
                    <p>你可能需要重启 gg-friendly 才能继续。</p>
                    <RecentWorkspaces workspaces={recentWorkspaces} />
                </ErrorDialog>
            </ModalOverlay>
        {:else}
            <ModalOverlay>
                <ErrorDialog title="出错了" severe>
                    <p>和后台的连接断了：{$repoConfigEvent.message}</p>
                    <p>你可能需要重启 gg-friendly 才能继续。</p>
                    <RecentWorkspaces workspaces={recentWorkspaces} />
                </ErrorDialog>
            </ModalOverlay>
        {/if}

        <div class="separator" style="grid-row: 3"></div>

        <StatusBar {target} />

        {#if $currentInput}
            <ModalOverlay>
                <InputDialog
                    title={$currentInput.title}
                    detail={$currentInput.detail}
                    fields={$currentInput.fields}
                    on:response={(event) => $currentInput?.callback(event.detail)} />
            </ModalOverlay>
        {:else if $currentMutation}
            <ModalOverlay>
                {#if $currentMutation.type == "wait" && $progressEvent !== undefined}
                    <ProgressDialog progress={$progressEvent} />
                {:else if $currentMutation.type == "data" && ($currentMutation.value.type == "InternalError" || $currentMutation.value.type == "PreconditionError")}
                    <ErrorDialog title="操作没能完成" onClose={() => ($currentMutation = null)} severe>
                        {#if $currentMutation.value.type == "InternalError"}
                            <p>
                                {#each $currentMutation.value.message.lines as line}
                                    {line}<br />
                                {/each}
                            </p>
                        {:else}
                            <p>{$currentMutation.value.message}</p>
                        {/if}
                    </ErrorDialog>
                {:else if $currentMutation.type == "error"}
                    <ErrorDialog title="连接出错" onClose={() => ($currentMutation = null)} severe>
                        <p>{$currentMutation.message}</p>
                    </ErrorDialog>
                {/if}
            </ModalOverlay>
        {/if}

        {#if $currentContext && $hasMenu}
            <ContextMenu
                operand={$currentContext}
                x={$hasMenu.x}
                y={$hasMenu.y}
                onClose={() => {
                    hasMenu.set(null);
                    currentContext.set(null);
                }} />
        {/if}
    </div>
</Zone>

<style>
    #shell {
        width: 100vw;
        height: 100vh;

        display: grid;
        grid-template-columns: 1fr;
        grid-template-rows: auto 1fr 3px 30px;
        grid-template-areas:
            "banner"
            "content"
            "."
            "footer";

        background: var(--ctp-crust);
        color: var(--ctp-text);

        user-select: none;
    }

    /* the banner slot is absent until a workspace opens, so the content
       takes the top row when there's no banner to sit above it */
    .banner {
        grid-area: banner;
    }

    .content {
        grid-area: content;
        overflow: hidden;
        min-height: 0;
    }

    .separator {
        background: var(--ctp-overlay0);
    }

    p {
        pointer-events: auto;
        user-select: text;
    }
</style>
