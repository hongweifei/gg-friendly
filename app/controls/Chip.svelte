<script lang="ts">
    export let context: boolean;
    export let target: boolean;
    export let disconnected: boolean = false;
    export let immobile: boolean = false;
    export let tip: string = "";
</script>

<div class="chip" class:context class:target class:disconnected class:immobile title={tip}>
    <slot />
</div>

<style>
    .chip {
        pointer-events: auto;

        font-family: var(--stack-code);
        font-size: smaller;

        height: var(--height-compact);
        line-height: 16px;

        display: flex;
        align-items: center;
        border: 1px solid var(--ctp-overlay1);
        border-radius: var(--radius-control);
        padding: 0 8px;
        background: var(--ctp-mantle);
        white-space: nowrap;
        gap: 4px;
    }

    .context {
        color: inherit;
        border-color: currentColor;
    }

    .target {
        background: var(--ctp-blue);
        color: white;
    }

    .disconnected {
        border-style: dashed;
    }

    .immobile {
        border-style: none;
    }
</style>
