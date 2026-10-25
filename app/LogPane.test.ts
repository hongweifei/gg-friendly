import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from "vitest";
import { render, waitFor } from "@testing-library/svelte";
import type { LogPage } from "./messages/LogPage";
import { setupMocks, cleanupMocks } from "./mocks";

describe("LogPane", () => {
    beforeAll(() => {
        setupMocks((cmd, _args) => {
            if (cmd === "query_log") {
                let emptyPage: LogPage = { rows: [], has_more: false };
                return emptyPage;
            }
            return undefined;
        });
    });

    afterAll(async () => {
        await cleanupMocks();
    });

    it("renders loading state", async () => {
        const { default: LogPane } = await import("./LogPane.svelte");

        const { container } = render(LogPane, {
            props: {
                query_choices: { default: "all()" },
                latest_query: "all()",
            },
        });

        expect(container.textContent).toContain("正在加载改动...");
    });

    it("renders empty log with mocked IPC", async () => {
        const { default: LogPane } = await import("./LogPane.svelte");
        const { repoStatusEvent } = await import("./stores");

        const { container } = render(LogPane, {
            props: {
                query_choices: { default: "all()" },
                latest_query: "all()",
            },
        });

        // the list only queries once the workspace is loaded
        repoStatusEvent.set({
            operation_description: "test",
            working_copy: { type: "CommitId", hex: "abc", prefix: "abc", rest: "" },
            can_redo: false,
        });

        await waitFor(() => {
            expect(container.textContent).not.toContain("正在加载改动...");
        });
    });
});

describe("LogPane (query requests)", () => {
    let revsets: string[];

    beforeEach(() => {
        revsets = [];
        setupMocks((cmd, args) => {
            if (cmd === "query_log") {
                revsets.push(args.revset as string);
                let emptyPage: LogPage = { rows: [], has_more: false };
                return emptyPage;
            }
            return undefined;
        });
    });

    afterEach(async () => {
        await cleanupMocks();
    });

    it("a logQueryRequest repoints the log and shows up in the query dropdown", async () => {
        const { default: LogPane } = await import("./LogPane.svelte");
        const { logQueryRequest, repoStatusEvent } = await import("./stores");
        const { get } = await import("svelte/store");
        logQueryRequest.set(null);

        const { container } = render(LogPane, {
            props: {
                query_choices: { default: "all()" },
                latest_query: "all()",
            },
        });

        // the list only queries once the workspace is loaded
        repoStatusEvent.set({
            operation_description: "test",
            working_copy: { type: "CommitId", hex: "abc", prefix: "abc", rest: "" },
            can_redo: false,
        });

        await waitFor(() => expect(revsets).toEqual(["all()"]));

        logQueryRequest.set('files("a.txt")');

        // the requested query is combined with the active filter
        await waitFor(() =>
            expect(revsets).toEqual(["all()", '(all()) & (files("a.txt"))']),
        );

        // the request is one-shot, and the new query is shown in the dropdown
        expect(get(logQueryRequest)).toBe(null);
        let select = container.querySelector("select")!;
        expect(select.value).toBe('files("a.txt")');
    });

    it("the list is read again when the repository changes", async () => {
        const { default: LogPane } = await import("./LogPane.svelte");
        const { repoChanges, repoStatusEvent } = await import("./stores");
        const { get } = await import("svelte/store");

        render(LogPane, {
            props: {
                query_choices: { default: "all()" },
                latest_query: "all()",
            },
        });

        repoStatusEvent.set({
            operation_description: "test",
            working_copy: { type: "CommitId", hex: "abc", prefix: "abc", rest: "" },
            can_redo: false,
        });

        await waitFor(() => expect(revsets).toEqual(["all()"]));

        // an operation that leaves the working copy alone, like copying a
        // change, still has to bring the list up to date
        repoChanges.set(get(repoChanges) + 1);

        await waitFor(() => expect(revsets).toEqual(["all()", "all()"]));
    });
});
