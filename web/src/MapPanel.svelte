<script lang="ts">
  import { widthLimits, widthColorScale, tool, colors, controls } from "./";
  import { SequentialLegend, QualitativeLegend } from "svelte-utils";
  import DebuggerLegend from "./DebuggerLegend.svelte";
  import type { FeatureCollection } from "geojson";

  export let debuggedFace: FeatureCollection;
  export let debuggedEdge: FeatureCollection;
</script>

<div>
  <details open>
    <summary>Layers</summary>
    <label>
      <u>S</u>
      how original OSM
      <input
        type="checkbox"
        role="switch"
        bind:checked={$controls.showSimplified}
      />
      <u>S</u>
      how simplified graph
    </label>

    <br />

    <label>
      <input type="checkbox" bind:checked={$controls.showEdges} />
      Show edges
    </label>

    <label>
      <input type="checkbox" bind:checked={$controls.showIntersections} />
      Show intersections
    </label>

    <label>
      <input type="checkbox" bind:checked={$controls.showBuildings} />
      Show building centroids
    </label>

    <hr />

    <QualitativeLegend
      labelColors={{
        "urban block": colors.UrbanBlock,
        "road artifact": colors.RoadArtifact,
        "sidepath artifact": colors.SidepathArtifact,
        "other area": colors.OtherArea,
      }}
      itemsPerRow={3}
    />

    <label>
      <input type="checkbox" bind:checked={$controls.showFaces} />
      Show faces
    </label>

    <label>
      <input type="checkbox" bind:checked={$controls.showUrbanBlocks} />
      Show urban blocks
    </label>

    <hr />

    <QualitativeLegend
      labelColors={{
        ...colors.edges,
        Intersection: colors.Intersection,
      }}
      itemsPerRow={2}
    />

    <hr />

    {#if $tool == "width"}
      <SequentialLegend
        colorScale={widthColorScale}
        labels={{ limits: widthLimits }}
      />
    {/if}

    <DebuggerLegend data={debuggedFace} />
    <DebuggerLegend data={debuggedEdge} />
  </details>
</div>

<style>
  div {
    background: white;
    padding: 16px;
  }
</style>
