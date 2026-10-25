import { describe, it, expect, beforeEach, afterEach } from "vitest";
import {
    TestContext,
    createContext,
    destroyContext,
    extractTestRepo,
    startGG,
} from "./harness.js";

describe("`gg web` in a jj workspace", () => {
    let ctx: TestContext;

    beforeEach(async () => {
        ctx = await createContext()
        extractTestRepo(ctx.tempDir);
    });

    afterEach(async () => {
        await destroyContext(ctx);
    });

    it("loads the workspace and displays the log", async () => {
        let serverUrl = startGG(ctx);

        await ctx.page.goto(await serverUrl);

        // wait for content to load
        await ctx.page.waitForTimeout(2000);

        // should NOT show the "没有打开仓库" error
        let errorDialog = ctx.page.locator("text=没有打开仓库");
        await expect(errorDialog.isVisible()).resolves.toBe(false);

        // the page should have a change list
        let logPaneSelector = ctx.page.locator(".filters");
        await expect(logPaneSelector.isVisible()).resolves.toBe(true);
    });
});
