<!--
@component
A row of tabs. Only one is active at a time, and the caller decides what
each tab shows.
-->

<script lang="ts" module>
    export interface Tab {
        id: string;
        label: string;
        /** a small count shown after the label, like "3 个文件" */
        hint?: string;
    }
</script>

<script lang="ts">
    export let tabs: Tab[];
    export let active: string;
</script>

<div class="tabs" role="tablist">
    {#each tabs as tab}
        <button
            role="tab"
            class="tab"
            class:on={tab.id == active}
            aria-selected={tab.id == active}
            on:click={() => (active = tab.id)}>
            <span class="label">{tab.label}</span>
            {#if tab.hint}
                <span class="hint">{tab.hint}</span>
            {/if}
        </button>
    {/each}
</div>

<style>
    .tabs {
        display: flex;
        align-items: stretch;
        gap: 2px;
        flex: none;
        border-bottom: 1px solid var(--ctp-surface0);
    }

    .tab {
        display: inline-flex;
        align-items: baseline;
        gap: 6px;
        height: 30px;
        padding: 0 12px;
        border: none;
        border-bottom: 2px solid transparent;
        border-radius: 0;
        background: none;
        color: var(--ctp-subtext1);
        font-family: var(--stack-text);
        font-size: var(--size-small-text);
        cursor: pointer;
        white-space: nowrap;
    }

    .tab:hover {
        color: var(--ctp-text);
    }

    .tab.on {
        color: var(--ctp-text);
        font-weight: 600;
        border-bottom-color: var(--ctp-lavender);
    }

    .tab:focus-visible {
        outline: 2px solid var(--ctp-lavender);
        outline-offset: -2px;
    }

    .hint {
        font-size: 11px;
        font-weight: 400;
        color: var(--ctp-subtext1);
    }
</style>
