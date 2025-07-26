<script lang="ts">
  import type { Amenity } from "../";

  export let list: Amenity[];
</script>

<details>
  <summary>{list.length} POIs</summary>
  <ol>
    {#each list as a}
      <li>
        <b>{a.kind}</b>
        : {a.name} ({a.brand}),
        <a
          href={`https://www.openstreetmap.org/node/${a.osm_id.Node}`}
          target="_blank"
        >
          OSM
        </a>
      </li>
    {/each}
  </ol>
</details>

<style>
  ol {
    max-height: 50vh;
    max-width: 30vw;
    overflow: auto;
  }
</style>
