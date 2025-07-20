<script lang="ts">
  import { RoadBundler } from "backend";
  import ToolSwitcher from "./ToolSwitcher.svelte";
  import { controls, backend, tool, type EdgeProps, type FaceProps } from "../";
  import type {
    LineString,
    FeatureCollection,
    Feature,
    Polygon,
  } from "geojson";
  import { downloadGeneratedFile } from "svelte-utils";

  export let undoCount: number;
  export let afterMutation: (undoDiff: number) => void;
  export let allRoadWidths: FeatureCollection;
  export let hoveredFace: Feature<Polygon, FaceProps> | null;
  export let edges: FeatureCollection<LineString, EdgeProps>;

  function keyDown(e: KeyboardEvent) {
    if (e.key == "z" && e.ctrlKey && undoCount > 0) {
      e.stopPropagation();
      undo();
    }

    if (e.key == "s") {
      $controls.showSimplified = !$controls.showSimplified;
    }
  }

  function undo() {
    try {
      $backend!.undo();

      afterMutation(-1);
    } catch (err) {
      window.alert(
        `You probably have to refresh the app now; something broke: ${err}`,
      );
    }
  }

  function doAllSimplifications() {
    doBulkEdit((b) => {
      return (
        b.removeAllSidepaths() +
        b.removeAllServiceRoads() +
        b.collapseAllDegenerateIntersections()
      );
    });
  }

  function doBulkEdit(cb: (b: RoadBundler) => number) {
    try {
      let newCommands = cb($backend!);
      afterMutation(newCommands);
    } catch (err) {
      window.alert(
        `You probably have to refresh the app now; something broke: ${err}`,
      );
    }
  }

  function getAllRoadWidths() {
    allRoadWidths = JSON.parse($backend!.getAllRoadWidths());
  }

  function downloadRoads() {
    downloadGeneratedFile(
      "roads.geojson",
      JSON.stringify({
        type: "FeatureCollection",
        features: edges.features.filter(
          (f) =>
            "Motorized" in f.properties.kind &&
            f.properties.kind.Motorized.roads.length > 0,
        ),
      }),
    );

    downloadGeneratedFile(
      "nonmotorized.geojson",
      JSON.stringify({
        type: "FeatureCollection",
        features: edges.features.filter(
          (f) => "Nonmotorized" in f.properties.kind,
        ),
      }),
    );
  }
</script>

<svelte:window on:keydown={keyDown} />

<div>
  <button class="secondary" on:click={undo} disabled={undoCount == 0}>
    Undo ({undoCount})
  </button>
</div>
<br />

<ToolSwitcher />

{#if $tool == "explore"}
  <p>Just pan around the map</p>
  <button class="outline" on:click={doAllSimplifications}>
    Do all simplifications
  </button>
{:else if $tool == "collapseToCentroid"}
  <p>Click to collapse a face to its centroid</p>
{:else if $tool == "dualCarriageway"}
  <p>Click to collapse a dual carriageway</p>

  <button
    class="outline"
    on:click={() => doBulkEdit((b) => b.fixAllDualCarriageways())}
  >
    Collapse all DCs
  </button>

  {#if hoveredFace}
    {#if typeof hoveredFace.properties.dual_carriageway == "string"}
      <p>
        Not a dual carriageway: {hoveredFace.properties.dual_carriageway}
      </p>
    {:else}
      {@const dc = hoveredFace.properties.dual_carriageway}
      <p>{dc.name}</p>
    {/if}
  {/if}
{:else if $tool == "sidepath"}
  <button
    class="outline"
    on:click={() => doBulkEdit((b) => b.removeAllSidepaths())}
  >
    Remove all sidepaths
  </button>

  <button class="outline" on:click={downloadRoads}>
    Download GJ of motorized and nonmotorized roads
  </button>
{:else if $tool == "dogleg"}
  <p>Click a dog-leg edge to collapse it</p>

  <button class="outline" on:click={() => doBulkEdit((b) => b.fixAllDogLegs())}>
    Collapse all dog-leg intersections
  </button>
{:else if $tool == "clean"}
  <p>Click an edge or degenerate intersection to collapse it</p>

  <button
    class="outline"
    on:click={() => doBulkEdit((b) => b.removeAllServiceRoads())}
  >
    Remove all service roads
  </button>

  <button
    class="outline"
    on:click={() => doBulkEdit((b) => b.collapseAllDegenerateIntersections())}
  >
    Collapse all degenerate intersections
  </button>
{:else if $tool == "width"}
  <p>Hover on an edge to measure its width</p>

  <button class="outline" on:click={getAllRoadWidths}>
    Get all road widths
  </button>
{/if}
