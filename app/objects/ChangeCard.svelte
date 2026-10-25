<!--
@component
One change, as a row of the commit table. The graph beside it draws the lanes.
-->

<script lang="ts">
    import type { RevHeader } from "../messages/RevHeader";
    import type { Operand } from "../messages/Operand";
    import TimestampSpan from "../controls/TimestampSpan.svelte";
    import IdSpan from "../controls/IdSpan.svelte";
    import AuthorSpan from "../controls/AuthorSpan.svelte";
    import { ignoreToggled } from "../stores";
    import { revisionTitle, branchChips, isUnpublished, isPublished, hasDescription } from "../vocabulary";
    import Object from "./Object.svelte";
    import Zone from "./Zone.svelte";
    import BookmarkObject from "./BookmarkObject.svelte";
    import TagObject from "./TagObject.svelte";
    import WorkspaceObject from "./WorkspaceObject.svelte";
    import RevisionMutator from "../mutators/RevisionMutator";

    export let header: RevHeader;
    export let selected: boolean;
    /** what the branch list knows about each branch */
    export let branchStates: Record<string, { ahead: boolean; synced: boolean; localOnly: boolean; absent: boolean }> = {};
    export let onClick: ((header: RevHeader) => void) | undefined = undefined;
    export let onShiftClick: ((header: RevHeader) => void) | undefined = undefined;

    $: operand = { type: "Revision", header } as Operand;
    $: title = revisionTitle(header);
    $: hasTitle = hasDescription(header);
    // a change with no description says so once, in the working copy's
    // case, and the status column carries the rest
    $: fallback = header.is_working_copy ? "正在编辑的工作副本" : "";
    $: branches = branchChips(header);
    $: unpublished = isUnpublished(header);
    $: published = isPublished(header);

    function onSelect(event: CustomEvent<MouseEvent>) {
        if (event.detail.shiftKey) {
            onShiftClick?.(header);
        } else {
            onClick?.(header);
        }
    }

    function onEdit() {
        new RevisionMutator([header], $ignoreToggled).onEdit();
    }
</script>

<Object
    {operand}
    suffix={header.id.commit.prefix}
    {selected}
    label={title}
    on:click={onSelect}
    on:dblclick={onEdit}
    let:context
    let:hint={dragHint}>
    <Zone {operand} let:target let:hint={dropHint}>
        <div class="row" class:target class:selected>
            <span class="msg" class:placeholder={!hasTitle} title={hasTitle ? title : fallback}>
                {dragHint ?? dropHint ?? (hasTitle ? title : fallback)}
            </span>

            <span class="who" class:unknown={!header.author.name}>
                {#if header.author.name}
                    <AuthorSpan author={header.author} />
                {:else}
                    未知作者
                {/if}
            </span>

            <span class="when">
                <TimestampSpan timestamp={header.author.timestamp} />
            </span>

            <span class="id">
                <IdSpan id={header.id.change} />
            </span>

            <span class="tags">
                {#each header.refs as ref}
                    {#if ref.type != "Tag"}
                        {#if ref.type == "LocalBookmark" || !ref.is_synced || !ref.is_tracked}
                            <BookmarkObject {header} {ref} />
                        {/if}
                    {:else}
                        <TagObject {header} {ref} />
                    {/if}
                {/each}
                {#if header.working_copy_of}
                    <WorkspaceObject name={header.working_copy_of} />
                {/if}
            </span>

            <span class="status">
                {#if header.is_working_copy}
                    <span class="tag ok">正在编辑</span>
                {/if}
                {#if header.has_conflict}
                    <span class="tag danger">有冲突</span>
                {/if}
                {#each header.refs as ref}
                    {#if ref.type == "LocalBookmark"}
                        {@const state = branchStates[ref.bookmark_name]}
                        {#if state?.ahead}
                            <span class="tag warn">未上传</span>
                        {:else if state?.synced}
                            <span class="tag ok">已上传</span>
                        {:else if state?.localOnly}
                            <span class="tag local">仅本地</span>
                        {/if}
                    {/if}
                {/each}
            </span>
        </div>
    </Zone>
</Object>

<style>
    .row {
        pointer-events: auto;
        display: grid;
        grid-template-columns: minmax(0, 1fr) 124px 76px 70px 150px 132px;
        align-items: center;
        gap: 10px;
        height: 100%;
        /* the graph sets this so the text clears the lanes */
        padding-left: var(--leftpad, 6px);
        padding-right: 10px;
        border-bottom: 1px solid transparent;
    }

    .row:hover {
        background-color: var(--ctp-crust);
    }

    .row.selected {
        background-color: var(--ctp-crust);
        box-shadow: inset 2px 0 0 var(--ctp-lavender);
    }

    .msg {
        font-size: 13.5px;
        color: var(--ctp-text);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .row.selected .msg {
        font-weight: 500;
    }

    .msg.placeholder {
        color: var(--ctp-subtext1);
        font-style: italic;
    }

    .who.unknown {
        color: var(--ctp-overlay1);
    }

    .who {
        font-size: 12.5px;
        color: var(--ctp-subtext1);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .when {
        font-size: 12.5px;
        color: var(--ctp-subtext1);
        white-space: nowrap;
    }

    .when :global(.timestamp) {
        color: var(--ctp-subtext1);
    }

    .id {
        flex: none;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .tags {
        display: flex;
        gap: 4px;
        justify-content: flex-start;
        min-width: 0;
        overflow: hidden;
    }

    .tag {
        font-size: 11px;
        line-height: 1;
        padding: 4px 7px;
        border-radius: var(--radius-small);
        background: var(--ctp-surface0);
        color: var(--ctp-text);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        min-width: 0;
    }

    .status {
        display: flex;
        gap: 4px;
        justify-content: flex-end;
        min-width: 0;
        overflow: hidden;
    }

    .tag.local {
        background: var(--ctp-surface0);
        color: var(--ctp-text);
    }

    .tag.warn {
        background: rgba(254, 100, 11, 0.16);
        color: var(--tag-warn);
    }

    .tag.ok {
        background: rgba(64, 160, 43, 0.16);
        color: var(--tag-ok);
    }

    .tag.danger {
        background: rgba(210, 15, 57, 0.16);
        color: var(--tag-danger);
    }

    .target {
        background: var(--ctp-flamingo);
        color: black;
    }
</style>
