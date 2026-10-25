<!--
@component
The window's top bar: identity, search, and the actions people reach for most.
-->

<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import Icon from "../controls/Icon.svelte";
    import { repoConfigEvent, searchOpen, searchText, themeChoice, type ThemeChoice } from "../stores";

    const dispatch = createEventDispatcher<{
        newChange: {};
        push: { remote: string };
        fetch: { remote: string };
    }>();

    $: repoName =
        $repoConfigEvent?.type == "Workspace"
            ? ($repoConfigEvent.absolute_path.split("/").filter(Boolean).pop() ?? "")
            : "";
    $: remotes = $repoConfigEvent?.type == "Workspace" ? $repoConfigEvent.git_remotes : [];

    let searchInput: HTMLInputElement;

    const themeOptions: { id: ThemeChoice; icon: string; title: string }[] = [
        { id: "system", icon: "monitor", title: "跟随系统" },
        { id: "light", icon: "sun", title: "浅色" },
        { id: "dark", icon: "moon", title: "深色" },
    ];

    function toggleSearch() {
        $searchOpen = !$searchOpen;
        if ($searchOpen) {
            requestAnimationFrame(() => searchInput?.focus());
        }
    }

    function onSearchKeydown(event: KeyboardEvent) {
        if (event.key == "Escape") {
            searchText.set("");
            $searchOpen = false;
            searchInput?.blur();
        }
    }
</script>

<header class="topbar">
    <div class="identity">
        <span class="mark" aria-hidden="true">G</span>
        <span class="product">gg-friendly</span>
        {#if repoName}
            <span class="repo" title={$repoConfigEvent?.type == "Workspace" ? $repoConfigEvent.absolute_path : ""}>
                {repoName}
            </span>
        {/if}
    </div>

    <div class="middle">
        {#if $searchOpen}
            <div class="search">
                <Icon name="search" />
                <input
                    type="search"
                    placeholder="搜索标题、作者或标识"
                    spellcheck="false"
                    bind:this={searchInput}
                    bind:value={$searchText}
                    on:keydown={onSearchKeydown} />
                <button class="close" title="关闭搜索" on:click={() => ($searchOpen = false)}>
                    <Icon name="x" />
                </button>
            </div>
        {:else}
            <button class="ghost search-open" title="搜索改动" on:click={toggleSearch}>
                <Icon name="search" />
                <span>搜索改动</span>
            </button>
        {/if}
    </div>

    <div class="actions">
        <div class="theme" role="group" aria-label="主题">
            {#each themeOptions as option}
                <button
                    type="button"
                    class="theme-button"
                    class:on={$themeChoice == option.id}
                    title={option.title}
                    aria-pressed={$themeChoice == option.id}
                    on:click={() => themeChoice.set(option.id)}>
                    <Icon name={option.icon} />
                </button>
            {/each}
        </div>

        {#each remotes as remote}
            <button class="ghost" title="把分支上传到 {remote}" on:click={() => dispatch("push", { remote })}>
                <Icon name="upload-cloud" />
                <span>上传</span>
            </button>
            <button class="ghost" title="从 {remote} 下载最新内容" on:click={() => dispatch("fetch", { remote })}>
                <Icon name="download-cloud" />
                <span>下载</span>
            </button>
        {/each}

        <button class="primary" title="新建一个改动" on:click={() => dispatch("newChange", {})}>
            <Icon name="plus" />
            <span>新建改动</span>
        </button>
    </div>
</header>

<style>
    .topbar {
        grid-area: banner;
        display: grid;
        grid-template-columns: auto minmax(0, 1fr) auto;
        align-items: center;
        gap: 16px;
        height: 52px;
        padding: 0 14px;
        background: var(--ctp-crust);
        border-bottom: 1px solid var(--ctp-surface0);
    }

    .identity {
        display: flex;
        align-items: center;
        gap: 9px;
        min-width: 0;
    }

    .mark {
        flex: none;
        width: 22px;
        height: 22px;
        border-radius: 6px;
        background: var(--ctp-lavender);
        color: var(--ctp-crust);
        display: grid;
        place-items: center;
        font-size: 12px;
        font-weight: 700;
        font-family: var(--stack-text);
    }

    .product {
        font-size: 14.5px;
        font-weight: 600;
        white-space: nowrap;
    }

    .repo {
        font-size: 13px;
        color: var(--ctp-subtext1);
        padding-left: 10px;
        margin-left: 2px;
        border-left: 1px solid var(--ctp-surface1);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        max-width: 220px;
    }

    .middle {
        display: flex;
        justify-content: flex-start;
        min-width: 0;
    }

    .search {
        display: flex;
        align-items: center;
        gap: 8px;
        width: 100%;
        max-width: 340px;
        height: var(--height-compact);
        padding: 0 8px 0 10px;
        background: var(--ctp-base);
        border: 1px solid var(--ctp-surface0);
        border-radius: var(--radius-control);
    }

    .search input {
        flex: 1;
        min-width: 0;
        height: 100%;
        border: none;
        background: none;
        font-family: var(--stack-text);
        font-size: 13.5px;
        color: var(--ctp-text);
    }

    .search input:focus-visible {
        outline: none;
    }

    .search :global(.feather) {
        color: var(--ctp-subtext0);
    }

    .close {
        border: none;
        background: none;
        padding: 2px;
        cursor: pointer;
        color: var(--ctp-subtext0);
        display: grid;
        place-items: center;
    }

    .close:hover {
        color: var(--ctp-text);
    }

    .actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .theme {
        display: flex;
        align-items: center;
        gap: 1px;
        padding: 2px;
        border-radius: var(--radius-control);
        border: 1px solid var(--ctp-surface0);
        background: var(--ctp-base);
    }

    .theme-button {
        width: 26px;
        height: 24px;
        padding: 0;
        border: none;
        border-radius: var(--radius-small);
        background: transparent;
        color: var(--ctp-subtext1);
        display: grid;
        place-items: center;
        cursor: pointer;
    }

    .theme-button:hover {
        background: var(--ctp-surface0);
        color: var(--ctp-text);
    }

    .theme-button.on {
        background: var(--ctp-lavender);
        color: #1b1f2b;
    }

    button {
        font-family: var(--stack-text);
        font-size: var(--size-control-text);
        height: var(--height-control);
        padding: 0 12px;
        border-radius: var(--radius-control);
        border: 1px solid var(--ctp-surface0);
        background: var(--ctp-base);
        color: var(--ctp-text);
        display: inline-flex;
        align-items: center;
        gap: 6px;
        cursor: pointer;
        white-space: nowrap;
    }

    button:hover:not(:disabled) {
        background: var(--ctp-surface0);
    }

    button:focus-visible {
        outline: 2px solid var(--ctp-lavender);
        outline-offset: 1px;
    }

    button:disabled {
        background: var(--ctp-mantle);
        color: var(--ctp-overlay2);
        cursor: default;
    }

    .primary {
        background: var(--ctp-blue);
        border-color: var(--ctp-blue);
        color: white;
        font-weight: 500;
    }

    .primary:hover:not(:disabled) {
        background: var(--ctp-sapphire);
    }

    .ghost {
        background: transparent;
        border-color: transparent;
        color: var(--ctp-subtext1);
    }

    .search-open {
        height: var(--height-compact);
        border: 1px solid var(--ctp-surface0);
        background: var(--ctp-base);
        color: var(--ctp-subtext1);
        min-width: 200px;
    }

    .ghost:hover:not(:disabled) {
        background: var(--ctp-surface0);
        color: var(--ctp-text);
    }
</style>
