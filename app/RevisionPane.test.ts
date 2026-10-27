import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import type { RevsResult } from "./messages/RevsResult";
import { setupMocks, cleanupMocks } from "./mocks";

/**
 * The details are in tabs, and the panel opens on the files. Switch to the
 * tab the test needs.
 */
async function showTab(container: HTMLElement, label: string) {
    let tab = [...container.querySelectorAll<HTMLButtonElement>(".tab")].find((t) =>
        t.textContent?.includes(label),
    );
    expect(tab, `no tab labelled ${label}`).toBeDefined();
    await fireEvent.click(tab!);
}

function createMockRevs(
    commitHex = "deadbeef1234",
    changeHex = "abc123def456",
    description = "Test commit message",
): Extract<RevsResult, { type: "Detail" }> {
    let mockId = {
        change: {
            type: "ChangeId" as const,
            hex: changeHex,
            prefix: changeHex.slice(0, 3),
            rest: changeHex.slice(3),
            offset: null,
            is_divergent: false,
        },
        commit: {
            type: "CommitId" as const,
            hex: commitHex,
            prefix: commitHex.slice(0, 4),
            rest: commitHex.slice(4),
        },
    };

    return {
        type: "Detail",
        set: {
            from: mockId,
            to: mockId,
        },
        headers: [
            {
                id: mockId,
                description: { lines: [description] },
                author: {
                    email: "test@example.com",
                    name: "Test User",
                    timestamp: "2024-01-15T12:00:00Z",
                },
                has_conflict: false,
                is_working_copy: false,
                working_copy_of: null,
                is_immutable: false,
                refs: [],
                parent_ids: [],
            },
        ],
        parents: [],
        changes: [],
        conflicts: [],
    };
}

describe("RevisionPane", () => {
    beforeEach(() => {
        setupMocks();
    });

    afterEach(async () => {
        await cleanupMocks();
    });

    it("renders revision details with mocked data", async () => {
        const { default: RevisionPane } = await import("./RevisionPane.svelte");

        let mockRevs = createMockRevs();
        const { container } = render(RevisionPane, {
            props: {
                revs: mockRevs,
            },
        });

        // the description and its metadata live in their own tab
        await showTab(container, "说明");

        // should display the change id
        expect(container.textContent).toContain("abc123de");

        // should display the commit id
        expect(container.textContent).toContain("deadbeef");

        // should display the description in the textarea
        let textarea = container.querySelector("textarea");
        expect(textarea).not.toBeNull();
        expect(textarea?.value).toBe("Test commit message");

        // should display the author
        expect(container.textContent).toContain("Test User");
    });

    it("opens on the files tab", async () => {
        const { default: RevisionPane } = await import("./RevisionPane.svelte");

        const { container } = render(RevisionPane, {
            props: { revs: createMockRevs() },
        });

        let active = container.querySelector(".tab.on");
        expect(active?.textContent).toContain("改动的文件");
    });

    it("preserves an undescribed message when the commit id changes", async () => {
        const { default: RevisionPane } = await import("./RevisionPane.svelte");

        let { container, rerender } = render(RevisionPane, {
            props: { revs: createMockRevs() },
        });

        await showTab(container, "说明");

        let textarea = container.querySelector("textarea")!;
        textarea.value = "desc";
        await fireEvent.input(textarea);

        await rerender({ revs: createMockRevs("feed5678abcd") });

        expect(container.querySelector("textarea")?.value).toBe("desc");
    });

    it("preserves an undescribed message when the pane is remounted", async () => {
        const { default: RevisionPane } = await import("./RevisionPane.svelte");

        let first = render(RevisionPane, { props: { revs: createMockRevs() } });
        await showTab(first.container, "说明");
        let textarea = first.container.querySelector("textarea")!;
        textarea.value = "desc";
        await fireEvent.input(textarea);
        first.unmount();

        let second = render(RevisionPane, { props: { revs: createMockRevs("feed5678abcd") } });
        await showTab(second.container, "说明");

        expect(second.container.querySelector("textarea")?.value).toBe("desc");
    });

    it("discards an undescribed message when the change id changes", async () => {
        const { default: RevisionPane } = await import("./RevisionPane.svelte");

        let first = render(RevisionPane, { props: { revs: createMockRevs() } });
        await showTab(first.container, "说明");
        let textarea = first.container.querySelector("textarea")!;
        textarea.value = "desc";
        await fireEvent.input(textarea);
        first.unmount();

        let second = render(RevisionPane, {
            props: { revs: createMockRevs("feed5678abcd", "999888777666", "Other message") },
        });
        await showTab(second.container, "说明");

        expect(second.container.querySelector("textarea")?.value).toBe("Other message");
    });
});
