<script lang="ts">
    import { dragOverWidget, hasModal } from "../stores";
    import Icon from "./Icon.svelte";

    export let tip: string = "";
    export let checked: boolean;
    export let safe: boolean = false;
    export let secondary: boolean = false;
    export let disabled: boolean = false;
    export let on: string;
    export let off: string;

    $: isDisabled = disabled || (!safe && $hasModal);

    function toggle() {
        if (!isDisabled) {
            checked = !checked;
        }
    }
</script>

<button
    {disabled}
    class:safe
    class:secondary
    class:checked
    on:click={toggle}
    on:dragenter={dragOverWidget}
    on:dragover={dragOverWidget}
    title={isDisabled ? "" : tip}>
    <Icon name={checked ? on : off} />
</button>

<style>
    button {
        width: 28px;
        height: var(--height-compact);
        padding: 0;

        outline: none;
        margin: 0;
        border: 1px solid var(--ctp-surface0);
        border-radius: var(--radius-control);

        background: var(--ctp-base);
        color: var(--ctp-subtext1);

        display: inline-flex;
        align-items: center;
        justify-content: center;

        cursor: pointer;
    }

    button:not(:disabled) {
        &:hover {
            background: var(--ctp-surface0);
            color: var(--ctp-text);
        }
        &:focus-visible {
            outline: 2px solid var(--ctp-lavender);
            outline-offset: 1px;
        }
    }

    button.checked:not(:disabled) {
        background: var(--ctp-lavender);
        border-color: var(--ctp-lavender);
        color: #1b1f2b;
    }

    button.safe:not(:disabled) {
        color: white;
        background: var(--ctp-blue);
        border-color: var(--ctp-blue);

        &:hover {
            background: var(--ctp-sapphire);
        }
    }

    button.secondary:not(:disabled) {
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
