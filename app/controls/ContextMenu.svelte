<!-- XXX duplicates a lot of dispatch and enablement logic from menu.rs, but it's not easy to share... -->
<script lang="ts">
    import type { Operand } from "../messages/Operand";
    import type { RevHeader } from "../messages/RevHeader";
    import type { StoreRef } from "../messages/StoreRef";
    import { ignoreToggled } from "../stores";
    import RefMutator from "../mutators/RefMutator";
    import { handleCommand } from "../mutators/commands";

    export let operand: Operand;
    export let x: number;
    export let y: number;
    export let onClose: () => void;

    // The menu keeps its own copy of the operand. Clicking a menu entry
    // clears the shared context before the entry's handler runs, so reading
    // the prop at that point would find nothing.
    let target: Operand = operand;

    function getRevisionHeaders(): RevHeader[] {
        if (target.type === "Revision") {
            return [target.header];
        } else if (target.type === "Revisions") {
            return target.headers;
        }
        return [];
    }

    function onClick(action: string) {
        handleCommand(target, action, $ignoreToggled);
        onClose();
    }

    function onDismiss() {
        onClose();
    }

    function onKeyDown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            onClose();
        }
    }

    function isRevisionEnabled(headers: RevHeader[], ignoreImmutable: boolean = false) {
        const isSingleton = headers.length == 1;
        const anyImmutable = !ignoreImmutable && headers.some((h) => h.is_immutable);
        const hasSingleParent = headers[headers.length - 1]?.parent_ids.length == 1;

        return {
            new_child: true,
            new_parent: !anyImmutable && hasSingleParent,
            edit: isSingleton && !anyImmutable && !headers[0]?.is_working_copy,
            revert: true,
            duplicate: true,
            abandon: !anyImmutable,
            squash: !anyImmutable && hasSingleParent,
            restore: isSingleton && !anyImmutable && hasSingleParent,
            bookmark: isSingleton,
        };
    }

    function isChangeEnabled(headers: RevHeader[], ignoreImmutable: boolean = false) {
        const anyImmutable = !ignoreImmutable && headers.some((h) => h.is_immutable);
        const hasSingleParent = headers[headers.length - 1]?.parent_ids.length == 1;

        return {
            squash: !anyImmutable && hasSingleParent,
            restore: !anyImmutable && hasSingleParent,
        };
    }

    function isRefEnabled(ref: StoreRef) {
        return {
            track: ref.type === "RemoteBookmark" && !ref.is_tracked,
            untrack:
                (ref.type === "LocalBookmark" && ref.tracking_remotes.length > 0) ||
                (ref.type === "RemoteBookmark" && !ref.is_synced && ref.is_tracked && !ref.is_absent),
            push_all:
                (ref.type === "LocalBookmark" && ref.tracking_remotes.length > 0) ||
                (ref.type === "RemoteBookmark" && ref.is_tracked && ref.is_absent),
            push_single: ref.type === "LocalBookmark" && ref.potential_remotes > 0,
            fetch_all:
                (ref.type === "LocalBookmark" && ref.tracking_remotes.length > 0) ||
                (ref.type === "RemoteBookmark" && (!ref.is_tracked || !ref.is_absent)),
            fetch_single: ref.type === "LocalBookmark" && ref.available_remotes > 0,
            rename: ref.type === "LocalBookmark",
            delete: !(ref.type === "RemoteBookmark" && ref.is_absent && ref.is_tracked),
        };
    }

    $: revisionEnabled =
        target.type === "Revision" || target.type === "Revisions"
            ? isRevisionEnabled(getRevisionHeaders(), $ignoreToggled)
            : null;
    $: changeEnabled = target.type === "Change" ? isChangeEnabled(target.headers, $ignoreToggled) : null;
    $: refEnabled = target.type === "Ref" ? isRefEnabled(target.ref) : null;
    $: defaultCommand = target.type === "Ref" ? RefMutator.defaultCommand(target.ref) : null;

    // clamp to viewport
    let menuElement: HTMLDivElement;
    $: if (menuElement) {
        const rect = menuElement.getBoundingClientRect();
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        if (x + rect.width > viewportWidth) {
            menuElement.style.left = `${viewportWidth - rect.width - 8}px`;
        }
        if (y + rect.height > viewportHeight) {
            menuElement.style.top = `${viewportHeight - rect.height - 8}px`;
        }
    }
</script>

<svelte:window on:click={onDismiss} />

<div
    class="context-menu"
    style="left: {x}px; top: {y}px;"
    role="menu"
    tabindex="0"
    on:click|stopPropagation
    on:keydown={onKeyDown}
    bind:this={menuElement}>
    {#if (target.type === "Revision" || target.type === "Revisions") && revisionEnabled}
        <button disabled={!revisionEnabled.new_child} on:click={() => onClick("new_child")}>新建子改动</button>
        <button disabled={!revisionEnabled.new_parent} on:click={() => onClick("new_parent")}
            >在上一个改动之前插入</button>
        <hr />
        <button disabled={!revisionEnabled.edit} on:click={() => onClick("edit")}>编辑这个改动</button>
        <button disabled={!revisionEnabled.revert} on:click={() => onClick("revert")}
            >把改动内容恢复到工作副本</button>
        <button disabled={!revisionEnabled.duplicate} on:click={() => onClick("duplicate")}>复制一份</button>
        <button disabled={!revisionEnabled.abandon} on:click={() => onClick("abandon")}>放弃这个改动</button>
        <hr />
        <button disabled={!revisionEnabled.squash} on:click={() => onClick("squash")}>合并到上一个改动</button>
        <button disabled={!revisionEnabled.restore} on:click={() => onClick("restore")}>从上一个改动恢复</button>
        <hr />
        <button disabled={!revisionEnabled.bookmark} on:click={() => onClick("bookmark")}>新建分支...</button>
    {:else if target.type === "Change" && changeEnabled}
        <button disabled={!changeEnabled.squash} on:click={() => onClick("squash")}>合并到上一个改动</button>
        <button disabled={!changeEnabled.restore} on:click={() => onClick("restore")}>从上一个改动恢复</button>
        <hr />
        <button on:click={() => onClick("file_history")}>查看这个文件的历史</button>
    {:else if target.type === "Ref" && refEnabled}
        <button
            disabled={!refEnabled.track}
            class:defaulted={defaultCommand === "track"}
            on:click={() => onClick("track")}>开始跟踪远程分支</button>
        <button
            disabled={!refEnabled.untrack}
            class:defaulted={defaultCommand === "untrack"}
            on:click={() => onClick("untrack")}>停止跟踪远程分支</button>
        <hr />
        <button disabled={!refEnabled.push_all} on:click={() => onClick("push-all")}>上传</button>
        <button disabled={!refEnabled.push_single} on:click={() => onClick("push-single")}>上传到指定远程...</button>
        <button disabled={!refEnabled.fetch_all} on:click={() => onClick("fetch-all")}>下载</button>
        <button disabled={!refEnabled.fetch_single} on:click={() => onClick("fetch-single")}
            >从指定远程下载...</button>
        <hr />
        <button disabled={!refEnabled.rename} on:click={() => onClick("rename")}>重命名...</button>
        <button disabled={!refEnabled.delete} on:click={() => onClick("delete")}>删除</button>
    {:else if target.type === "Workspace"}
        <button class="defaulted" on:click={() => onClick("open")}>打开</button>
        <button on:click={() => onClick("forget")}>从列表中移除</button>
        <button on:click={() => onClick("rename")}>重命名...</button>
    {/if}
</div>

<style>
    .context-menu {
        position: fixed;
        z-index: 1000;
        background: var(--ctp-base);
        border: 1px solid var(--ctp-surface0);
        border-radius: var(--radius-control);
        box-shadow: 0 6px 20px rgba(0, 0, 0, 0.22);
        padding: 4px;
    }

    hr {
        border: none;
        border-top: 1px solid var(--ctp-surface0);
        margin: 4px 6px;
    }

    button {
        width: 100%;
        display: block;
        border: none;
        padding: 6px 10px;
        border-radius: var(--radius-small);

        text-align: left;
        background: none;
        color: var(--ctp-text);
        font-family: var(--stack-text);
        font-size: var(--size-control-text);
        line-height: 1.4;

        &:disabled {
            color: var(--ctp-overlay1);
        }

        /* the command run by double-clicking the object */
        &.defaulted:not(:disabled) {
            font-weight: 600;
        }

        &:not(:disabled) {
            cursor: pointer;
            &:hover {
                background: var(--ctp-surface0);
            }
        }
    }
</style>
