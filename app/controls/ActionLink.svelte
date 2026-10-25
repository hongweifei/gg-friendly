<script lang="ts">
    import { dragOverWidget } from "../stores";

    export let tip: string = "";
    export let onClick: (event: MouseEvent) => void;
</script>

<button on:click|stopPropagation={onClick} on:dragenter={dragOverWidget} on:dragover={dragOverWidget} title={tip}>
    <slot />
</button>

<style>
    button {
        height: var(--height-compact);
        padding: 0;
        margin: 0;

        background: none;
        border: none;
        border-radius: var(--radius-small);

        color: var(--ctp-sapphire);
        font-family: var(--stack-text);
        font-size: var(--size-small-text);

        display: inline-flex;
        align-items: center;
        cursor: pointer;
    }

    button:not(:disabled) {
        &:hover {
            color: var(--ctp-teal);
        }
        &:focus-visible {
            outline: 2px solid var(--ctp-lavender);
            outline-offset: 1px;
        }
    }

    button:disabled {
        color: var(--ctp-overlay2);
    }
</style>
