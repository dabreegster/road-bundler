<script lang="ts">
  import { CircleLayer, GeoJSON, LineLayer } from "svelte-maplibre";
  import type { FeatureCollection } from "geojson";
  import { isPoint, isLine } from "svelte-utils/map";

  export let name: string;
  export let data: FeatureCollection;
</script>

<GeoJSON {data}>
  <LineLayer
    id={`debug-${name}-lines`}
    beforeId="Road labels"
    filter={isLine}
    paint={{
      "line-width": ["get", "width"],
      "line-color": ["get", "color"],
      "line-opacity": ["get", "opacity"],
    }}
  />

  <CircleLayer
    id={`debug-${name}-points`}
    beforeId="Road labels"
    filter={isPoint}
    paint={{
      "circle-color": ["get", "color"],
      "circle-radius": ["get", "radius"],
    }}
  />
</GeoJSON>
