<!--
@component
The list of branches. This is the panel people look at first to see where the
repository stands, so it leads with the branch being worked on.
-->

<script lang="ts">
    import type { BranchInfo } from "../messages/BranchInfo";
    import Icon from "../controls/Icon.svelte";

    export let branches: BranchInfo[] = [];
    export let selectedBranch: string | null = null;
    export let workingCopyId: string | null = null;
    /** the branches the selected change is on */
    export let selectedBranchNames: Set<string> = new Set();
    export let onSelect: ((branch: BranchInfo) => void) | undefined = undefined;
    export let onNewBranch: (() => void) | undefined = undefined;

    $: localBranches = branches.filter((b) => b.is_local);
    $: remoteBranches = branches.filter((b) => !b.is_local);

    /** the name a branch is shown under, with the remote for remote branches */
    function displayName(branch: BranchInfo): string {
        return branch.is_local ? branch.name : `${branch.name}@${branch.remote_name}`;
    }

    /** a local branch that tracks a remote it differs from has changes to upload */
    function isAhead(branch: BranchInfo): boolean {
        return branch.is_local && branch.is_tracking && !branch.is_synced;
    }

    /** a local branch that matches the remote it tracks has been uploaded */
    function isSynced(branch: BranchInfo): boolean {
        return branch.is_local && branch.is_tracking && branch.is_synced;
    }

    /** a local branch with no remote to compare against */
    function isLocalOnly(branch: BranchInfo): boolean {
        return branch.is_local && !branch.is_tracking;
    }
</script>

<aside class="branches">
    <div class="head">
        <span class="grow">分支</span>
        <button class="new" title="新建分支" on:click={() => onNewBranch?.()}>
            <Icon name="plus" />
        </button>
    </div>

    <div class="list">
        {#if workingCopyId}
            <div class="working">
                <span class="dot current"></span>
                <span class="name">正在编辑</span>
            </div>
        {/if}

        {#each localBranches as branch}
            <button
                class="branch"
                class:on={selectedBranch == displayName(branch)}
                class:related={selectedBranchNames.has(branch.name)}
                title={isAhead(branch) ? `${branch.name}（有改动还没上传）` : branch.name}
                on:click={() => onSelect?.(branch)}>
                <span
                    class="dot"
                    class:ahead={isAhead(branch)}
                    class:conflict={branch.has_conflict}></span>
                <span class="name">{branch.name}</span>
                {#if isAhead(branch)}
                    <span class="mark ahead">未上传</span>
                {:else if isSynced(branch)}
                    <span class="mark ok">已上传</span>
                {:else if isLocalOnly(branch)}
                    <span class="mark local">仅本地</span>
                {/if}
            </button>
        {/each}

        {#if remoteBranches.length > 0}
            <div class="group">远程分支</div>
            {#each remoteBranches as branch}
                <button
                    class="branch remote"
                    class:on={selectedBranch == displayName(branch)}
                    class:gone={branch.is_absent}
                    title={branch.is_absent
                        ? `${displayName(branch)}（远程上已经没有这个分支了）`
                        : displayName(branch)}
                    on:click={() => onSelect?.(branch)}>
                    <span class="dot" class:conflict={branch.has_conflict}></span>
                    <span class="name">
                        {displayName(branch)}
                        {#if branch.is_absent}
                            <span class="gone-mark">已删除</span>
                        {/if}
                    </span>
                </button>
            {/each}
        {/if}

        {#if branches.length == 0}
            <div class="empty">这个仓库里还没有分支</div>
        {/if}
    </div>
</aside>

<style>
    .branches {
        display: grid;
        grid-template-rows: auto 1fr;
        min-height: 0;
        background: var(--ctp-base);
        border-right: 1px solid var(--ctp-overlay0);
    }

    .head {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 9px 10px 7px 12px;
        font-size: 11.5px;
        font-weight: 600;
        letter-spacing: 0.4px;
        color: var(--ctp-subtext1);
    }

    .head .grow {
        flex: 1;
    }

    .new {
        border: none;
        background: none;
        padding: 2px 4px;
        border-radius: 4px;
        color: var(--ctp-subtext1);
        cursor: pointer;
        display: grid;
        place-items: center;
    }

    .new:hover {
        background: var(--ctp-surface0);
        color: var(--ctp-text);
    }

    .list {
        overflow-y: auto;
        padding-bottom: 12px;
    }

    .group {
        padding: 12px 12px 4px;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.4px;
        color: var(--ctp-subtext1);
    }

    .working {
        display: grid;
        grid-template-columns: 10px minmax(0, 1fr);
        align-items: center;
        gap: 8px;
        padding: 5px 12px 5px 11px;
        font-size: 13.5px;
        font-weight: 600;
        color: var(--ctp-text);
    }

    .dot.current {
        border-color: var(--ctp-green);
        background: var(--ctp-green);
    }

    .branch {
        display: grid;
        grid-template-columns: 10px minmax(0, 1fr);
        align-items: center;
        gap: 8px;
        width: 100%;
        padding: 5px 12px 5px 11px;
        border: none;
        background: none;
        color: var(--ctp-text);
        font-size: 13.5px;
        text-align: left;
        cursor: pointer;
    }

    .branch:hover {
        background: var(--ctp-crust);
    }

    .branch.on {
        background: var(--ctp-crust);
        box-shadow: inset 2px 0 0 var(--ctp-lavender);
    }

    /* the branch the selected change is on */
    .branch.related .name {
        font-weight: 600;
    }

    .branch.on .name {
        font-weight: 600;
    }

    .branch.remote {
        font-size: 12.5px;
        color: var(--ctp-text);
    }

    .branch.gone .name {
        text-decoration: line-through;
    }

    .gone-mark {
        font-size: 10.5px;
        margin-left: 5px;
        padding: 1px 5px;
        border-radius: var(--radius-small);
        background: var(--ctp-surface0);
        color: var(--ctp-text);
        text-decoration: none;
        display: inline-block;
    }

    .dot {
        width: 9px;
        height: 9px;
        border-radius: 50%;
        background: transparent;
        border: 2px solid var(--ctp-overlay1);
    }

    .dot.ahead {
        border-color: var(--ctp-peach);
        background: var(--ctp-peach);
    }

    .dot.conflict {
        border-color: var(--ctp-red);
        background: var(--ctp-red);
    }

    .name {
        overflow-wrap: anywhere;
        line-height: 1.35;
    }

    .mark {
        grid-column: 2;
        justify-self: start;
        font-size: 11px;
        padding: 2px 7px;
        border-radius: var(--radius-small);
        white-space: nowrap;
    }

    .mark.ahead {
        background: rgba(254, 100, 11, 0.16);
        color: var(--tag-warn);
    }

    .mark.local {
        background: var(--ctp-surface0);
        color: var(--ctp-text);
    }

    .mark.ok {
        background: rgba(64, 160, 43, 0.16);
        color: var(--tag-ok);
    }

    .empty {
        padding: 16px 12px;
        font-size: 12.5px;
        color: var(--ctp-subtext1);
    }
</style>
