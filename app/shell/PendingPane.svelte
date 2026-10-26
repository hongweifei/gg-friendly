<!--
@component
The files waiting to be committed. This is the one thing people check most
often, so it stays on screen instead of behind a selection.
-->

<script lang="ts">
    import type { RevsResult } from "../messages/RevsResult";
    import { query } from "../ipc";
    import { repoChanges, repoStatusEvent } from "../stores";
    import ChangeObject from "../objects/ChangeObject.svelte";
    import Icon from "../controls/Icon.svelte";
    import { fileChangeLabel } from "../vocabulary";

    /** how many files are listed before the rest is counted */
    const MAX_FILES = 50;

    type Detail = Extract<RevsResult, { type: "Detail" }>;

    let detail: Detail | null = null;
    let loading = true;
    let error: string | null = null;

    // the working copy's own id, so the query asks for exactly that change
    $: workingCopy = $repoStatusEvent?.working_copy;
    $: workingCopyId = workingCopy?.hex ?? null;

    // the panel is read again whenever the repository changes, since that is
    // when the set of pending files can differ
    $: if (workingCopyId !== undefined && workingCopyId !== null) {
        loadPending(workingCopyId, $repoChanges);
    }

    async function loadPending(commitHex: string, _change: number) {
        loading = true;
        error = null;
        // the working copy is addressed by "@", which jj resolves to whatever
        // the working copy currently is
        let id = {
            change: {
                type: "ChangeId" as const,
                hex: "@",
                prefix: "@",
                rest: "",
                offset: null,
                is_divergent: false,
            },
            commit: {
                type: "CommitId" as const,
                hex: commitHex,
                prefix: commitHex.slice(0, 2),
                rest: commitHex.slice(2),
            },
        };
        let result = await query<RevsResult>("query_revisions", { set: { from: id, to: id } });
        if (result.type === "data") {
            if (result.value.type == "Detail") {
                detail = result.value;
            } else {
                detail = null;
            }
        } else {
            error = result.message;
        }
        loading = false;
    }

    $: files = detail ? detail.changes.concat(
        detail.conflicts.map((conflict) => ({
            kind: "None" as const,
            path: conflict.path,
            has_conflict: true,
            hunks: [conflict.hunk],
        })),
    ) : [];
    $: shown = files.slice(0, MAX_FILES);
    $: restCount = files.length - shown.length;
</script>

<section class="pending">
    <div class="head">
        <Icon name="edit-3" />
        <span class="title">待提交改动</span>
        <span class="grow"></span>
        {#if detail}
            <span class="count">{files.length} 个文件</span>
        {/if}
    </div>

    <div class="body">
        {#if loading && !detail}
            <p class="note">正在读取...</p>
        {:else if error}
            <p class="note error">{error}</p>
        {:else if files.length == 0}
            <p class="note">没有待提交的改动，工作副本是干净的。</p>
        {:else}
            <ul class="files">
                {#each shown as change}
                    <li>
                        <ChangeObject
                            headers={detail?.headers ?? null}
                            {change}
                            selected={false} />
                    </li>
                {/each}
            </ul>
            {#if restCount > 0}
                <p class="note">还有 {restCount} 个文件没有列出。</p>
            {/if}
        {/if}
    </div>
</section>

<style>
    .pending {
        display: grid;
        grid-template-rows: auto minmax(0, 1fr);
        min-height: 0;
        border-bottom: 1px solid var(--ctp-surface0);
        background: var(--ctp-base);
    }

    .head {
        display: flex;
        align-items: center;
        gap: 7px;
        padding: 7px 12px;
        font-size: 12.5px;
        font-weight: 600;
        color: var(--ctp-subtext1);
        border-bottom: 1px solid var(--ctp-surface0);
    }

    .head :global(.feather) {
        color: var(--ctp-subtext0);
    }

    .grow {
        flex: 1;
    }

    .count {
        font-weight: 400;
        font-size: 11.5px;
        color: var(--ctp-subtext1);
    }

    .body {
        overflow: auto;
        min-height: 0;
    }

    .files {
        list-style: none;
        margin: 0;
        padding: 4px 0;
    }

    .note {
        margin: 0;
        padding: 12px 14px;
        font-size: 12.5px;
        color: var(--ctp-subtext1);
    }

    .note.error {
        color: var(--tag-danger);
    }
</style>
