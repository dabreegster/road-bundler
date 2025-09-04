<script lang="ts">
  import { CircleLayer, GeoJSON, LineLayer, FillLayer } from "svelte-maplibre";
  import type { FeatureCollection } from "geojson";
  import { isPoint, isLine, isPolygon } from "svelte-utils/map";

  export let name: string;
  export let data: FeatureCollection;
</script>

<GeoJSON {data}>
  <FillLayer
    id={`debug-${name}-polygon-fill`}
    beforeId="Road labels"
    filter={isPolygon}
    paint={{
      "fill-color": ["get", "color"],
      "fill-opacity": 0.8,
    }}
  />

  <LineLayer
    id={`debug-${name}-polygon-outline`}
    beforeId="Road labels"
    filter={isPolygon}
    paint={{
      "line-width": 2,
      "line-color": "black",
    }}
  />

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
