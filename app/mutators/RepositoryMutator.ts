import { getInput, mutate, trigger } from "../ipc";
import type { CloneRepository } from "../messages/CloneRepository";
import type { InitRepository } from "../messages/InitRepository";
import type { RepoEvent } from "../messages/RepoEvent";

export default class RepositoryMutator {
    async handle(event: RepoEvent) {
        switch (event.type) {
            case "CloneURL":
                await this.#handleUrl();
                break;

            case "CloneConfirm":
                await this.#handleClone(event.url, event.path);
                break;

            case "InitConfirm":
                await this.#handleInit(event.path, event.has_git);
                break;
        }
    }

    async #handleUrl() {
        let fields = [
            { label: "仓库地址", choices: [] }
        ];

        let result = await getInput("克隆仓库", "", fields);
        if (!result) return;

        let url = result["仓库地址"].trim();

        trigger("forward_clone_url", { url });
    }

    async #handleClone(originalUrl: string, originalPath: string) {
        let fields = [
            { label: "仓库地址", choices: [originalUrl] },
            { label: "保存到", choices: [originalPath] },
            { label: "同时作为 Git 仓库", choices: ["false", "true"] },
        ];

        let result = await getInput("克隆仓库", "", fields);
        if (!result) return;

        let url = result["仓库地址"].trim();
        let path = result["保存到"].trim();
        let colocated = result["同时作为 Git 仓库"] === "true";

        mutate<CloneRepository>("clone_repository", {
            url,
            path,
            colocated,
        }, { operation: "正在克隆仓库..." });
    }

    async #handleInit(originalPath: string, hasGit: boolean) {
        let fields = [
            { label: "保存到", choices: [originalPath] },
            { label: "同时作为 Git 仓库", choices: hasGit ? ["true", "false"] : ["false", "true"] },
        ];

        let result = await getInput("初始化仓库", "", fields);
        if (!result) return;

        let path = result["保存到"];
        let colocated = result["同时作为 Git 仓库"] === "true";

        mutate<InitRepository>("init_repository", {
            path,
            colocated,
        });
    }
}
