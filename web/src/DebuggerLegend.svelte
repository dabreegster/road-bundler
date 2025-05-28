<script lang="ts">
  import type { FeatureCollection } from "geojson";
  import { QualitativeLegend } from "svelte-utils";

  export let data: FeatureCollection;

  function getColors(data: FeatureCollection): Record<string, string> {
    let colors = {} as Record<string, string>;
    for (let f of data.features) {
      colors[f.properties!.label] = f.properties!.color;
    }
    return colors;
  }
</script>

<QualitativeLegend labelColors={getColors(data)} itemsPerRow={1} />
