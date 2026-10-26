<script lang="ts">
    import { parseRoute } from "./route.js";
    import Shell from "./Shell.svelte";
    import LogPane from "./LogPane.svelte";
    import RevisionPane from "./RevisionPane.svelte";
    import BoundQuery from "./controls/BoundQuery.svelte";
    import Pane from "./shell/Pane.svelte";
    import TitlebarInset from "./shell/TitlebarInset.svelte";
    import SetSpan from "./controls/SetSpan.svelte";
    import TopBar from "./shell/TopBar.svelte";
    import BranchPane from "./shell/BranchPane.svelte";
    import PendingPane from "./shell/PendingPane.svelte";
    import type { BranchInfo } from "./messages/BranchInfo";
    import type { RepoStatus } from "./messages/RepoStatus";
    import { branchFilter, ignoreToggled, logQueryRequest, repoChanges, repoStatusEvent, revisionSelectEvent } from "./stores.js";
    import { getInput, mutate, query } from "./ipc.js";
    import type { CreateRevision } from "./messages/CreateRevision";
    import type { GitPush } from "./messages/GitPush";
    import type { GitFetch } from "./messages/GitFetch";
    import type { CreateRef } from "./messages/CreateRef";
    import type { StoreRef } from "./messages/StoreRef";
    import type { RevId } from "./messages/RevId";
    import { get } from "svelte/store";

    let route = parseRoute();

    let branches: BranchInfo[] = [];
    let selectedBranch: string | null = null;
    // the working copy is its own change, not on any branch
    $: workingCopyId = $repoStatusEvent?.working_copy.hex ?? null;

    // the branch list is asked for separately, so it holds every branch in
    // the repository and not just the ones the change list happened to show
    async function loadBranches() {
        let result = await query<BranchInfo[]>("query_branches", null);
        if (result.type === "data") {
            branches = result.value;
        }
    }

    // The branch list is read again whenever the repository changes. The
    // change counter is what says so: renaming a branch or copying a change
    // leaves the working copy alone, so the status looks the same afterwards.
    // The workspace has to be loaded first, or the backend can't answer.
    let lastRepoChanges = -1;
    $: if ($repoStatusEvent && $repoChanges !== lastRepoChanges) {
        lastRepoChanges = $repoChanges;
        loadBranches();
    }

    /**
     * What the branch list knows about each branch, so the table and the
     * list never disagree about whether a branch is uploaded.
     */
    $: branchStates = Object.fromEntries(
        branches.map((b) => [
            b.is_local ? b.name : `${b.name}@${b.remote_name}`,
            {
                ahead: b.is_local && b.is_tracking && !b.is_synced,
                synced: b.is_local && b.is_tracking && b.is_synced,
                localOnly: b.is_local && !b.is_tracking,
                absent: b.is_absent,
            },
        ]),
    );

    /**
     * The local branches that have been uploaded and the ones that haven't.
     * The change list's upload filters are built from these, so the filters
     * and the row labels always agree.
     */
    $: publishedBranches = branches
        .filter((b) => b.is_local && b.is_tracking && b.is_synced)
        .map((b) => b.name);
    $: unpublishedBranches = branches
        .filter((b) => b.is_local && b.is_tracking && !b.is_synced)
        .map((b) => b.name);

    /**
     * Picking a branch narrows the change list to it, the way other git
     * tools do. Picking it again clears the narrowing.
     */
    function onSelectBranch(branch: BranchInfo) {
        let name = branch.is_local ? branch.name : `${branch.name}@${branch.remote_name}`;

        // picking the same branch again clears the narrowing
        if (selectedBranch == name) {
            selectedBranch = null;
            branchFilter.set(null);
            return;
        }

        selectedBranch = name;
        let escaped = branch.name.replace(/["\\]/g, "\\$&");
        branchFilter.set(
            branch.is_local
                ? `bookmarks(exact:"${escaped}")`
                : `remote_bookmarks(exact:"${escaped}", exact:"${branch.remote_name}")`,
        );
    }

    /**
     * A new change is created on top of whatever is selected, so the button
     * does the same thing as "new child" on the selected change.
     */
    function onPush(event: CustomEvent<{ remote: string }>) {
        mutate<GitPush>(
            "git_push",
            { refspec: { type: "AllBookmarks", remote_name: event.detail.remote }, input: null },
            { operation: `正在上传到 ${event.detail.remote}...` },
        );
    }

    function onFetch(event: CustomEvent<{ remote: string }>) {
        mutate<GitFetch>(
            "git_fetch",
            { refspec: { type: "AllBookmarks", remote_name: event.detail.remote }, input: null },
            { operation: `正在从 ${event.detail.remote} 下载...` },
        );
    }

    /**
     * Creating a branch needs a name and a place to put it. The place is the
     * change that is selected, or the working copy if nothing is selected.
     */
    async function onNewBranch() {
        // the branch goes on the change that is selected, or on the working
        // copy if nothing is selected
        let selection = get(revisionSelectEvent);
        let id: RevId | null = selection?.from ?? null;
        if (!id) {
            let wc = $repoStatusEvent?.working_copy;
            if (!wc) {
                return;
            }
            id = {
                change: {
                    type: "ChangeId",
                    hex: "",
                    prefix: "",
                    rest: "",
                    offset: null,
                    is_divergent: false,
                },
                commit: wc,
            };
        }

        let response = await getInput("新建分支", "", ["分支名称"]);
        if (!response) {
            return;
        }

        let name = response["分支名称"]?.trim();
        if (!name) {
            return;
        }

        let ref: StoreRef = {
            type: "LocalBookmark",
            bookmark_name: name,
            has_conflict: false,
            is_synced: false,
            potential_remotes: 0,
            available_remotes: 0,
            tracking_remotes: [],
        };

        mutate<CreateRef>("create_ref", { ref, id: id as RevId }, { ignoreImmutable: $ignoreToggled });
        await loadBranches();
    }

    function onNewChange() {
        let selection = get(revisionSelectEvent);
        if (!selection) {
            logQueryRequest.set("");
            return;
        }
        mutate<CreateRevision>("create_revision", { set: selection });
    }
</script>

<Shell revsetOverride={route.type === "revision" ? route.revset : null} let:workspace let:selection let:selectedBranchNames>
    <TopBar slot="banner" on:newChange={onNewChange} on:push={onPush} on:fetch={onFetch} />

    {#if route.type === "log"}
        <!-- key on the whole config, not its path - reopening the same repo must remount -->
        {#key workspace}
            <LogPane query_choices={workspace.query_choices} latest_query={route.revset ?? workspace.latest_query} />
        {/key}
    {:else if route.type === "revision"}
        <BoundQuery query={selection} let:data>
            {#if data.type == "Detail"}
                <RevisionPane revs={data} />
            {:else}
                <Pane>
                    <h2 slot="header">没有找到</h2>
                    <p slot="body">
                        空的改动集合 <SetSpan set={data.set} />。
                    </p>
                </Pane>
            {/if}
            <Pane slot="error" let:message>
                <h2 slot="header">出错了</h2>
                <p slot="body">{message}</p>
            </Pane>
            <Pane slot="wait">
                <h2 slot="header">正在加载...</h2>
            </Pane>
        </BoundQuery>
    {:else}
        <div class="workspace">
            <TitlebarInset>
                <BranchPane
                {branches}
                {selectedBranch}
                {workingCopyId}
                {selectedBranchNames}
                onSelect={onSelectBranch}
                onNewBranch={onNewBranch} />
            </TitlebarInset>

            <div class="main">
                <!-- key on the whole config, not its path - reopening the same repo must remount -->
                {#key workspace}
                    <LogPane
                        query_choices={workspace.query_choices}
                        latest_query={workspace.latest_query}
                        {branchStates}
                        {publishedBranches}
                        {unpublishedBranches} />
                {/key}

                <div class="divider" role="separator" aria-orientation="horizontal"></div>

                <div class="detail">
                    <PendingPane />

                    <div class="divider" role="separator" aria-orientation="horizontal"></div>

                    <BoundQuery query={selection} let:data>
                        {#if data.type == "Detail"}
                            <RevisionPane revs={data} />
                        {:else}
                            <Pane titlebar>
                                <h2 slot="header">没有找到</h2>
                                <p slot="body">
                                    空的改动集合 <SetSpan set={data.set} />。
                                </p>
                            </Pane>
                        {/if}
                        <Pane titlebar slot="error" let:message>
                            <h2 slot="header">出错了</h2>
                            <p slot="body">{message}</p>
                        </Pane>
                        <Pane titlebar slot="wait">
                            <h2 slot="header">正在加载...</h2>
                        </Pane>
                    </BoundQuery>
                </div>
            </div>
        </div>
    {/if}
</Shell>

<style>
    .workspace {
        display: grid;
        grid-template-columns: 210px minmax(0, 1fr);
        height: 100%;
        overflow: hidden;
    }

    .main {
        display: grid;
        /* the change list gets a third of the height, the files and the diff
           below it get the rest */
        grid-template-rows: minmax(0, 1fr) 1px minmax(0, 1.6fr);
        overflow: hidden;
        min-width: 0;
    }

    .detail {
        display: grid;
        /* the pending files take a slice of the height, the selected change's
           details get the rest */
        grid-template-rows: minmax(0, 0.7fr) 1px minmax(0, 1.6fr);
        overflow: hidden;
        min-height: 0;
    }

    .divider {
        background: var(--ctp-overlay0);
    }
</style>
