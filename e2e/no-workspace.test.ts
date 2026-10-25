import { describe, it, expect, beforeEach, afterEach } from "vitest";
import {
    TestContext,
    createContext,
    destroyContext,
    startGG,
} from "./harness.js";

describe("`gg web` in an empty directory", () => {
    let ctx: TestContext;

    beforeEach(async () => {
        ctx = await createContext();
    });

    afterEach(async () => {
        await destroyContext(ctx);
    });

    it("displays '没有打开仓库'", async () => {
        let serverUrl = startGG(ctx);

        await ctx.page.goto(await serverUrl);

        // wait for the error dialog to appear
        await expect(async () => {
            let element = ctx.page.locator("text=没有打开仓库");
            await element.waitFor({ timeout: 15000 });
        }).not.toThrow();

        // verify it's got some of the expected content
        await expect(ctx.page.locator("text=请在 Jujutsu 仓库目录里运行").isVisible()).resolves.toBe(true);
    });
});
