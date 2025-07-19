<script lang="ts">
  import { notNull, PropertiesTable } from "svelte-utils";
  import type { Feature } from "geojson";

  export let collection: string;
  export let edges: number[];
  export let originalEdges: Record<number, Feature>;
</script>

{#if edges.length > 0}
  <u>{collection}</u>

  <!-- TODO nicer scroller -->
  {#each edges as e}
    {@const orig = notNull(originalEdges[e].properties)}
    <details>
      <summary>Original edge {e}</summary>
      <a href={`https://www.openstreetmap.org/way/${orig.way}`} target="_blank">
        Way {orig.way}
      </a>

      <PropertiesTable properties={orig.tags} />
    </details>
  {/each}
{/if}
