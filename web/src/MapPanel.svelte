<script lang="ts">
  import {
    poiLimits,
    poiColorScale,
    widthLimits,
    widthColorScale,
    tool,
    colors,
    controls,
  } from "./";
  import { Checkbox, SequentialLegend, QualitativeLegend } from "svelte-utils";
  import DebuggerLegend from "./DebuggerLegend.svelte";
  import type { FeatureCollection } from "geojson";

  export let debuggedFace: FeatureCollection;
  export let debuggedEdge: FeatureCollection;
</script>

<div>
  <details open>
    <summary>Layers</summary>
    <div class="fs-6">
      <div class="form-check form-switch">
        <input
          class="form-check-input"
          type="checkbox"
          role="switch"
          id="showSimplified"
          bind:checked={$controls.showSimplified}
        />
        <label class="form-check-label" for="showSimplified">
          <kbd>S</kbd>
          how simplified graph
        </label>
      </div>

      <Checkbox bind:checked={$controls.showEdges}>Show edges</Checkbox>

      <Checkbox bind:checked={$controls.showIntersections}>
        Show intersections
      </Checkbox>

      <Checkbox bind:checked={$controls.showBuildings}>
        Show building centroids
      </Checkbox>

      <hr />

      {#if $tool == "poi"}
        <SequentialLegend
          colorScale={poiColorScale}
          labels={{ limits: poiLimits }}
        />
      {:else}
        <QualitativeLegend
          labelColors={{
            "urban block": colors.UrbanBlock,
            "road artifact": colors.RoadArtifact,
            "sidepath artifact": colors.SidepathArtifact,
            "other area": colors.OtherArea,
          }}
          itemsPerRow={2}
        />
      {/if}

      <Checkbox bind:checked={$controls.showFaces}>Show faces</Checkbox>

      <Checkbox bind:checked={$controls.showUrbanBlocks}>
        Show urban blocks
      </Checkbox>

      <hr />

      {#if $tool != "poi"}
        <QualitativeLegend
          labelColors={{
            ...colors.edges,
            Intersection: colors.Intersection,
          }}
          itemsPerRow={1}
        />

        <hr />
      {/if}

      {#if $tool == "width"}
        <SequentialLegend
          colorScale={widthColorScale}
          labels={{ limits: widthLimits }}
        />
      {/if}

      <DebuggerLegend data={debuggedFace} />
      <DebuggerLegend data={debuggedEdge} />
    </div>
  </details>
</div>

<style>
  div {
    background: white;
    padding: 16px;
  }
</style>
