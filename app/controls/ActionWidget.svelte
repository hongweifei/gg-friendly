<script lang="ts">
    import { dragOverWidget, hasModal } from "../stores";

    export let tip: string = "";
    export let onClick: (event: MouseEvent) => void;
    export let safe: boolean = false;
    export let secondary: boolean = false;
    export let disabled: boolean = false;

    $: isDisabled = disabled || (!safe && $hasModal);
</script>

<button
    disabled={isDisabled}
    class:safe
    class:secondary
    on:click|stopPropagation={isDisabled ? undefined : onClick}
    on:dragenter={dragOverWidget}
    on:dragover={dragOverWidget}
    title={isDisabled ? "" : tip}>
    <slot />
</button>

<style>
    button {
        height: var(--height-compact);
        font-size: var(--size-small-text);
        font-family: var(--stack-text);
        padding: 0 10px;

        outline: none;
        margin: 0;
        border: 1px solid var(--ctp-surface0);
        border-radius: var(--radius-control);

        display: inline-flex;
        align-items: center;
        gap: 5px;

        cursor: pointer;
        white-space: nowrap;

        color: var(--ctp-text);
        background: var(--ctp-base);
    }

    button:not(:disabled) {
        &:hover {
            background: var(--ctp-surface0);
        }
        &:focus-visible {
            outline: 2px solid var(--ctp-lavender);
            outline-offset: 1px;
        }
    }

    button.safe {
        color: white;
        background: var(--ctp-blue);
        border-color: var(--ctp-blue);

        &:hover {
            background: var(--ctp-sapphire);
        }
    }

    button.secondary {
        color: var(--ctp-text);
        background: var(--ctp-surface0);
        border-color: var(--ctp-surface1);

        &:hover {
            background: var(--ctp-surface1);
        }
    }

    button:disabled {
        background: var(--ctp-mantle);
        color: var(--ctp-overlay2);
        cursor: default;
    }
</style>
