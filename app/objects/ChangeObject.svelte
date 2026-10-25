<script lang="ts">
    import type { RevChange } from "../messages/RevChange";
    import type { RevHeader } from "../messages/RevHeader";
    import type { Operand } from "../messages/Operand";
    import type { ExternalDiff } from "../messages/ExternalDiff";
    import type { ExternalResolve } from "../messages/ExternalResolve";
    import Icon from "../controls/Icon.svelte";
    import ActionWidget from "../controls/ActionWidget.svelte";
    import Object from "./Object.svelte";
    import Zone from "./Zone.svelte";
    import { changeSelectEvent, repoConfigEvent } from "../stores";
    import { mutate } from "../ipc";
    import ActionLink from "../controls/ActionLink.svelte";
    import { fileChangeLabel } from "../vocabulary";

    export let headers: RevHeader[] | null;
    export let change: RevChange;
    export let selected: boolean;

    let operand: Operand | null = headers ? { type: "Change", headers, path: change.path, hunk: null } : null;

    $: hasDiffTool = $repoConfigEvent.type === "Workspace" && $repoConfigEvent.has_external_diff_tool;
    $: hasMergeTool = $repoConfigEvent.type === "Workspace" && $repoConfigEvent.has_external_merge_tool;

    function onSelect() {
        changeSelectEvent.set(change);
    }

    function onExternalDiff() {
        if (!headers) return;
        mutate<ExternalDiff>("external_diff", {
            id: headers[0].id,
            path: change.path,
        });
    }

    function onExternalResolve() {
        if (!headers) return;
        mutate<ExternalResolve>("external_resolve", {
            id: headers[0].id,
            path: change.path,
        });
    }
</script>

<Object
    {operand}
    {selected}
    suffix={change.path.repo_path}
    conflicted={change.has_conflict}
    label={change.path.relative_path}
    on:click={onSelect}
    let:context
    let:hint>
    <Zone {operand} let:target>
        <div class="layout" class:target>
            <span class="kind {change.has_conflict ? "danger" : change.kind.toLowerCase()}">
                {change.has_conflict ? "冲突" : fileChangeLabel(change.kind)}
            </span>
            <span class="path">{hint ?? change.path.relative_path}</span>
            {#if change.has_conflict}
                <span class="flag">有冲突</span>
            {/if}
            {#if hasMergeTool && change.has_conflict && operand}
                <ActionWidget tip="用合并工具解决" onClick={onExternalResolve}>
                    <Icon name="external-link" /> 解决
                </ActionWidget>
            {:else if hasDiffTool && operand}
                <ActionLink tip="在差异工具里打开" onClick={onExternalDiff}>
                    <Icon name="external-link" />
                </ActionLink>
            {/if}
        </div>
    </Zone>
</Object>

<style>
    .layout {
        min-height: 30px;
        display: flex;
        align-items: center;
        gap: 8px;
        padding-left: 3px;
    }

    .kind {
        font-size: 11.5px;
        line-height: 1;
        padding: 4px 8px;
        border-radius: var(--radius-small);
        background: var(--ctp-surface0);
        color: var(--ctp-text);
        flex: none;
    }

    .kind.added {
        background: rgba(64, 160, 43, 0.16);
        color: var(--tag-ok);
    }

    .kind.deleted {
        background: rgba(210, 15, 57, 0.16);
        color: var(--tag-danger);
    }

    .kind.modified {
        background: rgba(30, 102, 245, 0.14);
        color: var(--tag-info);
    }

    .kind.danger {
        background: rgba(210, 15, 57, 0.16);
        color: var(--tag-danger);
    }

    .path {
        flex: 1;
        font-family: var(--stack-code);
        font-size: 12.5px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .flag {
        font-size: 11.5px;
        color: var(--tag-danger);
        flex: none;
    }

    .layout.target {
        background: var(--ctp-flamingo);
        color: black;
    }
</style>
