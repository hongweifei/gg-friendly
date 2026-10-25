<!--
@component
The list of changes. Filters sit above the cards so people can narrow the
history without typing a revset expression.
-->

<script lang="ts">
    import type { LogPage } from "./messages/LogPage.js";
    import type { LogRow } from "./messages/LogRow.js";
    import type { RevHeader } from "./messages/RevHeader";
    import type { RepoStatus } from "./messages/RepoStatus";
    import { getInput, query, trigger } from "./ipc.js";
    import { sameChange } from "./ids.js";
    import { ignoreToggled, logQueryRequest, repoChanges, repoStatusEvent, revisionSelectEvent, searchOpen, searchText, branchFilter } from "./stores.js";
    import RevisionMutator from "./mutators/RevisionMutator.js";
    import Pane from "./shell/Pane.svelte";
    import ChangeCard from "./objects/ChangeCard.svelte";
    import ActionWidget from "./controls/ActionWidget.svelte";
    import Icon from "./controls/Icon.svelte";
    import SelectWidget from "./controls/SelectWidget.svelte";
    import ListWidget, { type List, type Selection } from "./controls/ListWidget.svelte";
    import { type EnhancedRow, default as GraphLog, ROW_HEIGHT } from "./GraphLog.svelte";
    import { branchesRevset } from "./vocabulary.js";

    export let query_choices: Record<string, string>;
    export let latest_query: string;
    export let branchStates: Record<string, { ahead: boolean; synced: boolean; localOnly: boolean; absent: boolean }> = {};
    /** the names of the local branches that have and haven't been uploaded */
    export let publishedBranches: string[] = [];
    export let unpublishedBranches: string[] = [];

    type FilterId = "all" | "mine" | "unpublished" | "conflicts" | "published";

    /**
     * The filters. The "已上传" and "未上传" ones are built from the same
     * branch list the row labels come from, so a labelled row is always
     * found by the matching filter.
     */
    const filters: { id: FilterId; label: string }[] = [
        { id: "all", label: "全部" },
        { id: "mine", label: "我参与的" },
        { id: "unpublished", label: "未上传" },
        { id: "conflicts", label: "有冲突" },
        { id: "published", label: "已上传" },
    ];

    let activeFilter: FilterId = "all";

    // the backend remembers the last revset it was given, which includes the
    // filter, so the dropdown starts from the preset instead
    let entered_query = query_choices["default"] ?? latest_query;

    function toTitleCase(kebab: string): string {
        return kebab
            .split("-")
            .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
            .join(" ");
    }

    /**
     * The built-in presets have names the backend chooses, so they're
     * translated here. Anything the user configured keeps its own name.
     */
    const presetNames: Record<string, string> = {
        "tracked-bookmarks": "已跟踪的分支",
        "remote-bookmarks": "远程分支",
        "all-revisions": "全部改动",
    };

    function presetLabel(key: string): string {
        return presetNames[key] ?? toTitleCase(key);
    }

    $: presets = (() => {
        let result: { label: string; value: string; separator?: boolean }[] = [];

        if (query_choices["default"] !== undefined) {
            result.push({ label: "默认", value: query_choices["default"] });
        }

        let others = Object.entries(query_choices)
            .filter(([key]) => key !== "default")
            .map(([key, value]) => ({ label: presetLabel(key), value }))
            .sort((a, b) => a.label.localeCompare(b.label));

        if (others.length > 0) {
            result.push({ label: "", value: "", separator: true });
            result.push(...others);
        }

        return result;
    })();

    let choices: ReturnType<typeof getChoices>;
    let graphRows: EnhancedRow[] | undefined;

    let selectionAnchorIdx: number | undefined;

    let logHeight = 0;
    let logWidth = 0;
    let logScrollTop = 0;

    function setSelection(anchorIdx: number | undefined, extendIdx: number) {
        if (!graphRows) return;

        if (anchorIdx !== undefined) {
            selectionAnchorIdx = anchorIdx;
        }

        const effectiveAnchor = selectionAnchorIdx ?? extendIdx;
        const fromIdx = Math.max(effectiveAnchor, extendIdx);
        const toIdx = Math.min(effectiveAnchor, extendIdx);

        $revisionSelectEvent = {
            from: graphRows[fromIdx].revision.id,
            to: graphRows[toIdx].revision.id,
        };
    }

    let list: List = {
        getSize() {
            return graphRows?.length ?? 0;
        },
        getSelection(): Selection {
            if (!graphRows || selectionAnchorIdx === undefined || !$revisionSelectEvent) {
                return { from: -1, to: -1 };
            }

            const revSetFromIdx = graphRows.findIndex(
                (row) => row.revision.id.commit.hex === $revisionSelectEvent!.from.commit.hex,
            );
            const revSetToIdx = graphRows.findIndex(
                (row) => row.revision.id.commit.hex === $revisionSelectEvent!.to.commit.hex,
            );

            const extensionIdx = revSetFromIdx === selectionAnchorIdx ? revSetToIdx : revSetFromIdx;

            return { from: selectionAnchorIdx, to: extensionIdx };
        },
        selectRow(row: number) {
            setSelection(row, row);
        },
        extendSelection(row: number) {
            if (!graphRows || selectionAnchorIdx === undefined) return;

            const limitIdx = findLinearLimit(selectionAnchorIdx, row);
            if (limitIdx === row) {
                setSelection(undefined, row);
            }
        },
        editRow(row: number) {
            if (row != -1) {
                new RevisionMutator([graphRows![row].revision], $ignoreToggled).onEdit();
            }
        },
    };

    $: choices = getChoices(entered_query, presets);
    $: if ($logQueryRequest) applyQueryRequest($logQueryRequest);

    // the top bar owns the search field, so the list follows it
    $: searchTerm = $searchText.trim();

    // the branch lists are compared by content: an array prop's identity
    // says nothing about whether its contents changed
    $: publishedKey = publishedBranches.join("\n");
    $: unpublishedKey = unpublishedBranches.join("\n");

    // The list is read again whenever the repository changes, and whenever
    // the search or the branch filter changes. The change counter is what
    // says the repository moved: the status looks the same after operations
    // that don't move the working copy, like copying a change.
    // The backend can't answer until the workspace is loaded.
    $: if (
        $repoStatusEvent &&
        ($repoChanges !== lastRepoChanges ||
            searchTerm !== lastSearchTerm ||
            $branchFilter !== lastBranchFilter ||
            publishedKey !== lastPublishedKey ||
            unpublishedKey !== lastUnpublishedKey)
    ) {
        lastRepoChanges = $repoChanges;
        lastSearchTerm = searchTerm;
        lastBranchFilter = $branchFilter;
        lastPublishedKey = publishedKey;
        lastUnpublishedKey = unpublishedKey;
        loadLog();
    }

    let lastRepoChanges = -1;
    let lastSearchTerm: string | null = null;
    let lastBranchFilter: string | null = null;
    let lastPublishedKey = "";
    let lastUnpublishedKey = "";

    // whether the list has been read at least once
    let hasLoaded = false;

    function applyQueryRequest(revset: string) {
        logQueryRequest.set(null);
        entered_query = revset;
        // the repo isn't queryable until the workspace has finished loading
        if ($repoStatusEvent) {
            reloadLog();
        }
    }

    function isInSelectedRange(row: EnhancedRow, selection: typeof $revisionSelectEvent): boolean {
        if (!selection || !graphRows) return false;
        const fromIdx = graphRows.findIndex((r) => r.revision.id.commit.hex === selection.from.commit.hex);
        const toIdx = graphRows.findIndex((r) => r.revision.id.commit.hex === selection.to.commit.hex);
        const rowIdx = graphRows.indexOf(row);
        if (fromIdx === -1 || toIdx === -1 || rowIdx === -1) return false;
        const minIdx = Math.min(fromIdx, toIdx);
        const maxIdx = Math.max(fromIdx, toIdx);
        return rowIdx >= minIdx && rowIdx <= maxIdx;
    }

    /**
     * Check if childRow's revision is a direct (non-elided) parent of parentRow's
     * revision, so shift-click can extend a selection along one line of history.
     */
    function isDirectParent(childRow: EnhancedRow, parentRow: EnhancedRow): boolean {
        const childCommitHex = childRow.revision.id.commit.hex;
        const parentCommitHex = parentRow.revision.id.commit.hex;

        if (!childRow.revision.parent_ids.some((p) => p.hex === parentCommitHex)) {
            return false;
        }

        // find a connecting line
        for (const line of childRow.passingLines) {
            if (line.child.id.commit.hex === childCommitHex && line.parent.id.commit.hex === parentCommitHex) {
                return !line.indirect && line.type !== "ToMissing";
            }
        }

        // elided sequences not supported for now - this is possible, but perhaps not useful
        return false;
    }

    function findLinearLimit(anchorIdx: number, targetIdx: number): number {
        if (!graphRows) return anchorIdx;

        const direction = targetIdx > anchorIdx ? 1 : -1;
        let lastValidIdx = anchorIdx;

        for (let i = anchorIdx + direction; direction > 0 ? i <= targetIdx : i >= targetIdx; i += direction) {
            const checkStart = direction > 0 ? lastValidIdx : i;
            const checkEnd = direction > 0 ? i : lastValidIdx;
            if (isDirectParent(graphRows[checkStart], graphRows[checkEnd])) {
                lastValidIdx = i;
            } else {
                break;
            }
        }

        return lastValidIdx;
    }

    function handleClick(header: RevHeader) {
        if (!graphRows) return;

        const clickedIdx = graphRows.findIndex((r) => r.revision.id.commit.hex === header.id.commit.hex);
        if (clickedIdx !== -1) {
            setSelection(clickedIdx, clickedIdx);
        }
    }

    function handleShiftClick(header: RevHeader) {
        if (!graphRows || selectionAnchorIdx === undefined) {
            handleClick(header);
            return;
        }

        const clickedIdx = graphRows.findIndex((r) => r.revision.id.commit.hex === header.id.commit.hex);
        if (clickedIdx === -1) {
            handleClick(header);
            return;
        }

        const limitIdx = findLinearLimit(selectionAnchorIdx, clickedIdx);
        setSelection(undefined, limitIdx);
    }

    /**
     * The dropdown lists the presets. A query that isn't one of them is
     * offered as an extra entry at the top, so the current selection is
     * always visible and picking it again doesn't change anything.
     */
    function getChoices(query: string, presetList: typeof presets) {
        for (let choice of presetList) {
            if (query == choice.value) {
                return presetList;
            }
        }

        return [{ label: "自定义", value: query }, ...presetList];
    }

    $: isCustom = !presets.some((p) => !p.separator && p.value === entered_query);

    $: isDeletable =
        !isCustom && presets.some((p) => !p.separator && p.value === entered_query && p.label !== "默认");

    function writePresets() {
        let values: Record<string, string> = {};
        for (let [key, value] of Object.entries(query_choices)) {
            if (key !== "default") {
                values[key] = value;
            }
        }
        trigger("write_config_table", { scope: "repo", key: ["gg", "presets"], values });
    }

    function toKebabCase(text: string): string {
        return text
            .trim()
            .toLowerCase()
            .replace(/\s+/g, "-")
            .replace(/[^a-z0-9-]/g, "");
    }

    async function handleSavePreset() {
        let response = await getInput("保存查询", "code:" + entered_query, ["查询名称"]);
        if (!response) return;

        let name = toKebabCase(response["查询名称"]);
        if (!name || name === "default") return;

        query_choices[name] = entered_query;
        query_choices = query_choices;
        writePresets();
    }

    function handleDeletePreset() {
        let keyToDelete = Object.entries(query_choices).find(
            ([key, value]) => key !== "default" && value === entered_query,
        )?.[0];
        if (!keyToDelete) return;

        delete query_choices[keyToDelete];
        query_choices = query_choices;
        entered_query = query_choices["default"] ?? "";
        writePresets();
        reloadLog();
    }

    /**
     * The revset behind one filter. The upload filters are built from the
     * branch list, so they pick out exactly the rows the labels mark.
     */
    function filterRevset(filter: FilterId): string {
        switch (filter) {
            case "unpublished":
                return branchesRevset(unpublishedBranches);
            case "published":
                return branchesRevset(publishedBranches);
            case "mine":
                return "mine()";
            case "conflicts":
                return "conflicts()";
            default:
                return "all()";
        }
    }

    /**
     * The revset sent to the backend. The chosen filter and the chosen query
     * are combined, and a search narrows the result further by title, author
     * or identifier.
     *
     * jj matches descriptions and author names with glob patterns, but ids
     * take a plain prefix, and a prefix that isn't a real id is an error
     * rather than an empty result.
     */
    function buildRevset(filter: FilterId, query: string): string {
        let base = filterRevset(filter);

        // the default query is what the filter already covers, so it isn't
        // added on top
        if (query && query != query_choices["default"]) {
            base = `(${base}) & (${query})`;
        }

        // picking a branch narrows the list to it
        if ($branchFilter) {
            base = `(${base}) & (${$branchFilter})`;
        }

        let term = $searchText.trim();
        if (!term) {
            return base;
        }
        let escaped = term.replace(/["\\]/g, "\\$&");
        let pattern = `*${escaped.replace(/[*?]/g, "")}*`;

        let terms = [`description("${pattern}")`, `author("${pattern}")`];

        // change ids use jj's own "reverse hex" digits, which run from k to
        // z, while commit ids are ordinary hex
        if (/^[k-z]{2,32}$/.test(term)) {
            terms.push(`change_id("${escaped}")`);
        } else if (/^[0-9a-f]{4,40}$/.test(term)) {
            terms.push(`commit_id("${escaped}")`);
        }

        return `(${base}) & (${terms.join(" | ")})`;
    }
    async function loadLog(selectFirst?: boolean) {
        // the first load has nothing selected yet, so it picks the first change
        let first = selectFirst ?? !hasLoaded;
        hasLoaded = true;
        let page = await query<LogPage>(
            "query_log",
            {
                revset: buildRevset(activeFilter, entered_query),
            },
            () => (graphRows = undefined),
        );

        if (page.type == "data") {
            graphRows = [];
            graphRows = addPageToGraph(graphRows, page.value.rows);

            if (first && page.value.rows.length > 0) {
                setSelection(0, 0);
            }

            while (page.value.has_more) {
                let next_page = await query<LogPage>("query_log_next_page", null);
                if (next_page.type == "data") {
                    graphRows = addPageToGraph(graphRows, next_page.value.rows);
                    page = next_page;
                } else {
                    break;
                }
            }

            if (!first) {
                syncSelectionWithGraph();
            }

        }
    }

    function syncSelectionWithGraph() {
        const selection = $revisionSelectEvent;
        if (!selection || !graphRows || graphRows.length === 0) {
            return;
        }

        let fromIdx = graphRows.findIndex((r) => r.revision.id.commit.hex === selection.from.commit.hex);
        let toIdx = graphRows.findIndex((r) => r.revision.id.commit.hex === selection.to.commit.hex);

        if (fromIdx === -1) {
            fromIdx = graphRows.findIndex((r) => sameChange(r.revision.id.change, selection.from.change));
        }
        if (toIdx === -1) {
            toIdx = graphRows.findIndex((r) => sameChange(r.revision.id.change, selection.to.change));
        }

        if (fromIdx !== -1 && toIdx !== -1) {
            selectionAnchorIdx = toIdx;

            const newFrom = graphRows[fromIdx].revision.id;
            const newTo = graphRows[toIdx].revision.id;
            if (newFrom.commit.hex !== selection.from.commit.hex || newTo.commit.hex !== selection.to.commit.hex) {
                $revisionSelectEvent = { from: newFrom, to: newTo };
            }
        } else {
            setSelection(0, 0);
        }
    }

    function reloadLog() {
        loadLog(false);
    }
    function onFilterChange(filter: FilterId) {
        activeFilter = filter;
        reloadLog();
    }

    let lineKey = 0;
    let passNextRow: any[] = [];
    function addPageToGraph(graph: EnhancedRow[], page: LogRow[]): EnhancedRow[] {
        for (let row of page) {
            let enhancedRow = row as EnhancedRow;
            for (let passingRow of passNextRow) {
                passingRow.parent = row.revision;
            }
            enhancedRow.passingLines = passNextRow;
            passNextRow = [];

            for (let line of enhancedRow.lines) {
                let enhancedLine = line as any;
                enhancedLine.key = lineKey++;

                if (line.type == "ToIntersection" || line.type == "ToMissing") {
                    enhancedLine.child = row.revision;
                    enhancedRow.passingLines.push(enhancedLine);
                    passNextRow.push(enhancedLine);
                } else {
                    enhancedLine.parent = row.revision;
                    enhancedLine.child = graph[line.source[1]].revision;
                    for (let i = line.source[1]; i < line.target[1]; i++) {
                        graph[i].passingLines.push(enhancedLine);
                    }
                    enhancedRow.passingLines.push(enhancedLine);
                }
            }

            graph.push(enhancedRow);
        }

        return graph;
    }
</script>

<Pane>
    <div slot="header" class="toolbar">
        <div class="filters" role="tablist" aria-label="筛选改动">
            {#each filters as filter}
                <button
                    type="button"
                    role="tab"
                    class="filter"
                    class:on={activeFilter == filter.id}
                    aria-selected={activeFilter == filter.id}
                    on:click={() => onFilterChange(filter.id)}>
                    {filter.label}
                </button>
            {/each}
        </div>

        <div class="tools">
            <div class="advanced">
                <SelectWidget options={choices} bind:value={entered_query} on:change={reloadLog}>
                    <svelte:fragment let:option>{option.label}</svelte:fragment>
                </SelectWidget>
                <input
                    type="text"
                    class="revset"
                    autocapitalize="off"
                    autocorrect="off"
                    autocomplete="off"
                    spellcheck="false"
                    placeholder="自定义查询"
                    title="可以自己写查询表达式"
                    bind:value={entered_query}
                    on:change={reloadLog} />
                {#if isCustom}
                    <ActionWidget secondary tip="保存查询" onClick={handleSavePreset}>
                        <Icon name="save" />
                    </ActionWidget>
                {:else if isDeletable}
                    <ActionWidget secondary tip="删除查询" onClick={handleDeletePreset}>
                        <Icon name="x-square" />
                    </ActionWidget>
                {/if}
            </div>
        </div>
    </div>

    <ListWidget
        slot="body"
        type="Revision"
        descendant={$revisionSelectEvent?.to.commit.prefix}
        {list}
        rowHeight={ROW_HEIGHT}
        bind:clientHeight={logHeight}
        bind:clientWidth={logWidth}
        bind:scrollTop={logScrollTop}>
        {#if graphRows}
            <div class="table-head">
                <span class="msg">说明</span>
                <span class="who">作者</span>
                <span class="when">时间</span>
                <span class="id">标识</span>
                <span class="tags">分支</span>
                <span class="status">状态</span>
            </div>
            <GraphLog
                containerHeight={logHeight}
                containerWidth={logWidth}
                scrollTop={logScrollTop}
                rows={graphRows}
                let:row>
                {#if row}
                    <ChangeCard
                        header={row.revision}
                        selected={isInSelectedRange(row, $revisionSelectEvent)}
                        {branchStates}
                        onClick={handleClick}
                        onShiftClick={handleShiftClick} />
                {/if}
            </GraphLog>
            {#if graphRows.length == 0}
                <div class="empty">
                    <p>没有符合条件的改动</p>
                    <p class="hint">换一个筛选条件，或清空搜索关键词试试。</p>
                </div>
            {/if}
        {:else}
            <div class="empty"><p>正在加载改动...</p></div>
        {/if}
    </ListWidget>
</Pane>

<style>
    .table-head {
        display: grid;
        grid-template-columns: minmax(0, 1fr) 124px 76px 70px 150px 132px;
        align-items: baseline;
        gap: 10px;
        padding: 5px 10px 5px 28px;
        font-size: 11.5px;
        font-weight: 600;
        letter-spacing: 0.3px;
        color: var(--ctp-subtext1);
        background: var(--ctp-crust);
        border-bottom: 1px solid var(--ctp-overlay0);
    }

    .toolbar {
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
    }

    .filters {
        display: flex;
        align-items: center;
        gap: 4px;
        overflow-x: auto;
        scrollbar-width: none;
    }

    .filters::-webkit-scrollbar {
        display: none;
    }

    .filter {
        height: var(--height-compact);
        padding: 0 11px;
        border-radius: var(--radius-control);
        font-family: var(--stack-text);
        font-size: var(--size-small-text);
        white-space: nowrap;
        border: 1px solid var(--ctp-surface0);
        background: var(--ctp-base);
        color: var(--ctp-subtext1);
        cursor: pointer;
    }

    .filter:hover {
        background: var(--ctp-surface0);
        color: var(--ctp-text);
    }

    .filter.on {
        background: var(--ctp-lavender);
        border-color: var(--ctp-lavender);
        color: #1b1f2b;
        font-weight: 500;
    }

    .tools {
        display: flex;
        align-items: center;
        gap: 8px;
        flex: none;
    }

    .advanced {
        display: flex;
        align-items: center;
        gap: 4px;
    }

    .revset {
        width: 190px;
        height: var(--height-compact);
        font-family: var(--stack-code);
        font-size: 12px;
    }

    .empty {
        padding: 48px 20px;
        text-align: center;
        color: var(--ctp-subtext0);
        font-size: 13.5px;
    }

    .empty p {
        margin: 0 0 6px;
    }

    .empty .hint {
        font-size: 12.5px;
        opacity: 0.85;
    }
</style>
