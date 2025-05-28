<script lang="ts">
  import { GeoJSON, LineLayer } from "svelte-maplibre";
  import type { FeatureCollection } from "geojson";

  export let data: FeatureCollection;
</script>

<GeoJSON {data}>
  <LineLayer
    beforeId="Road labels"
    paint={{
      "line-width": ["get", "width"],
      "line-color": ["get", "color"],
      "line-opacity": ["get", "opacity"],
    }}
  />
</GeoJSON>
