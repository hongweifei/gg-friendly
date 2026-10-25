<script lang="ts">
    import type { EnhancedLine } from "./GraphLog.svelte";
    import { COLUMN_WIDTH, ROW_HEIGHT } from "./GraphLog.svelte";
    import type { Operand } from "./messages/Operand";
    import Zone from "./objects/Zone.svelte";
    import { currentTarget } from "./stores";

    export let line: EnhancedLine;

    let isMerge = line.type == "ToIntersection";
    let allowEarlyBreak = line.type == "FromNode";
    let c1 = line.source[0];
    let c2 = line.target[0];
    let r1 = line.source[1];
    let r2 = line.target[1];

    let operand: Operand = { type: "Parent", header: line.parent, child: line.child };

    // the width of a lane, and the x of the node that sits in it
    const col = COLUMN_WIDTH;
    const lane = col - 4;
    const dotX = (c: number) => c * col + col / 2;
    const laneX = (c: number) => c * col + 2;

    // draw path downward, from child to parent
    let path: string;

    // the node sits near the top of its row, so the line leaves from just
    // below it and arrives just above the one it points to
    let childY = r1 * ROW_HEIGHT + 21;
    let parentY = r2 * ROW_HEIGHT + 9;

    type Block = { x: number; y: number; w: number; h: number };
    let blocks: Block[] = [];

    if (isMerge) {
        // instead of a parent, we have a mergepoint
        let childX = dotX(c1);
        let mergeX = dotX(c2);
        let midY = c2 > c1 ? childY + 9 : parentY - 9;
        let radius = c2 > c1 ? 6 : -6;
        let sweep = c2 > c1 ? 0 : 1;
        path = `M${childX},${childY}
            L${childX},${midY - 6}
            A6,6,0,0,${sweep},${childX + radius},${midY}
            L${mergeX - radius},${midY}
            A6,6,0,0,${1 - sweep},${mergeX},${midY + 6}
            L${mergeX},${parentY}`;

        blocks.push({
            x: c1 < c2 ? laneX(c1) : laneX(c2),
            y: midY - 8,
            w: c1 < c2 ? (c2 - c1 + 1) * col - 5 : (c1 - c2 + 1) * col - 5,
            h: 14,
        });
        // vertical segment at child column
        let topH = midY - 6 - childY;
        if (topH > 2) {
            blocks.push({ x: laneX(c1), y: childY, w: lane, h: topH });
        }
        // vertical segment at merge column
        let bottomH = parentY - (midY + 6);
        if (bottomH > 2) {
            blocks.push({ x: laneX(c2), y: midY + 6, w: lane, h: bottomH });
        }
    } else if (c1 == c2) {
        // same-column, straight line
        let x = dotX(c1);

        path = `M${x},${childY} L${x},${parentY}`;

        blocks.push({
            x: laneX(c1),
            y: childY,
            w: lane,
            h: parentY - childY,
        });
    } else {
        // different-column, curved line
        let childX = dotX(c1);
        let parentX = dotX(c2);
        let midY = allowEarlyBreak && c1 > c2 ? parentY - 9 : childY + 9;
        let radius = c2 > c1 ? 6 : -6;
        let sweep = c2 > c1 ? 0 : 1;
        path = `M${childX},${childY}
            L${childX},${midY - 6}
            A6,6,0,0,${sweep},${childX + radius},${midY}
            L${parentX - radius},${midY}
            A6,6,0,0,${1 - sweep},${parentX},${midY + 6}
            L${parentX},${parentY}`;

        // horizontal segment
        blocks.push({
            x: c1 < c2 ? c1 * col + 16 : c2 * col + 16,
            y: midY - 8,
            w: c1 < c2 ? (c2 - c1) * col - 14 : (c1 - c2) * col - 14,
            h: 14,
        });
        // vertical segment at child column
        let topH = midY - 6 - childY;
        if (topH > 2) {
            blocks.push({ x: laneX(c1), y: childY, w: lane, h: topH });
        }
        // vertical segment at parent column
        let bottomH = parentY - (midY + 6);
        if (bottomH > 2) {
            blocks.push({ x: laneX(c2), y: midY + 6, w: lane, h: bottomH });
        }
    }
</script>

{#if !line.indirect}
    {#each blocks as block}
        <foreignObject x={block.x} y={block.y} width={block.w} height={block.h}>
            <Zone {operand} let:target>
                <div class="backdrop" class:target></div>
            </Zone>
        </foreignObject>
    {/each}
{/if}

<path d={path} fill="none" stroke-dasharray={line.indirect ? "1,2" : "none"} class:target={$currentTarget == operand} />

<style>
    path {
        pointer-events: none;
        stroke: var(--ctp-blue);
    }

    foreignObject > :global(*) {
        height: 100%;
    }

    .backdrop {
        width: 100%;
        height: 100%;
    }

    .target {
        stroke: black;
        background-color: var(--ctp-flamingo);
    }
</style>
