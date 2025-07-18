<script lang="ts">
  import { RoadBundler } from "backend";
  import DebuggerLayer from "./DebuggerLayer.svelte";
  import ToolSwitcher from "./ToolSwitcher.svelte";
  import {
    widthColorScale,
    widthLimits,
    controls,
    tool,
    colors,
    backend,
    type IntersectionProps,
    type EdgeProps,
    type FaceProps,
  } from "./";
  import { SplitComponent } from "svelte-utils/two_column_layout";
  import { GeoJSON, LineLayer, CircleLayer, Control } from "svelte-maplibre";
  import type {
    LineString,
    Feature,
    FeatureCollection,
    Polygon,
    Point,
  } from "geojson";
  import {
    isLine,
    isPoint,
    emptyGeojson,
    Popup,
    makeRamp,
  } from "svelte-utils/map";
  import MapPanel from "./MapPanel.svelte";
  import Faces from "./layers/Faces.svelte";
  import Intersections from "./layers/Intersections.svelte";
  import Edges from "./layers/Edges.svelte";

  let edges: FeatureCollection<LineString, EdgeProps> = JSON.parse(
    $backend!.getEdges(),
  );
  let intersections: FeatureCollection<Point, IntersectionProps> = JSON.parse(
    $backend!.getIntersections(),
  );
  let faces: FeatureCollection<Polygon, FaceProps> = JSON.parse(
    $backend!.getFaces(),
  );
  let buildings: FeatureCollection<Point> = JSON.parse(
    $backend!.getBuildings(),
  );
  let originalGraph: FeatureCollection = JSON.parse(
    $backend!.getOriginalOsmGraph(),
  );

  let allRoadWidths: FeatureCollection = emptyGeojson();

  let undoCount = 0;

  let hoveredFace: Feature<Polygon, FaceProps> | null = null;
  let debuggedFace = emptyGeojson();
  let debuggedEdge = emptyGeojson();

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

  function afterMutation(undoDiff: number) {
    edges = JSON.parse($backend!.getEdges());
    intersections = JSON.parse($backend!.getIntersections());
    faces = JSON.parse($backend!.getFaces());
    hoveredFace = null;
    undoCount += undoDiff;
  }

  function keyDown(e: KeyboardEvent) {
    if (e.key == "z" && e.ctrlKey && undoCount > 0) {
      e.stopPropagation();
      undo();
    }

    if (e.key == "s") {
      $controls.showSimplified = !$controls.showSimplified;
    }
  }

  function getAllRoadWidths() {
    allRoadWidths = JSON.parse($backend!.getAllRoadWidths());
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
</script>

<svelte:window on:keydown={keyDown} />

<SplitComponent>
  <div slot="sidebar">
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
    {:else if $tool == "dogleg"}
      <p>Click a dog-leg edge to collapse it</p>

      <button
        class="outline"
        on:click={() => doBulkEdit((b) => b.fixAllDogLegs())}
      >
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
        on:click={() =>
          doBulkEdit((b) => b.collapseAllDegenerateIntersections())}
      >
        Collapse all degenerate intersections
      </button>
    {:else if $tool == "width"}
      <p>Hover on an edge to measure its width</p>

      <button class="outline" on:click={getAllRoadWidths}>
        Get all road widths
      </button>
    {/if}
  </div>

  <div slot="map">
    <Control position="top-right">
      <MapPanel {debuggedFace} {debuggedEdge} />
    </Control>

    <Faces {faces} bind:hoveredFace bind:debuggedFace {afterMutation} />

    <Edges {edges} {afterMutation} {originalGraph} bind:debuggedEdge />

    <Intersections {intersections} {afterMutation} />

    <GeoJSON data={buildings}>
      <CircleLayer
        id="buildings"
        paint={{
          "circle-color": colors.BuildingCentroid,
          "circle-radius": 3,
        }}
        layout={{ visibility: $controls.showBuildings ? "visible" : "none" }}
      />
    </GeoJSON>

    <GeoJSON data={allRoadWidths} generateId>
      <LineLayer
        id="road-widths"
        beforeId="edges"
        paint={{
          "line-width": 30,
          "line-color": makeRamp(
            ["get", "min_width"],
            widthLimits,
            widthColorScale,
          ),
        }}
        layout={{ visibility: $tool == "width" ? "visible" : "none" }}
      >
        <Popup openOn="hover" let:props>
          {Math.round(props.min_width)} to {Math.round(props.max_width)}
        </Popup>
      </LineLayer>
    </GeoJSON>

    {#if $backend}
      <GeoJSON data={originalGraph}>
        <LineLayer
          id="original-edges"
          beforeId="Road labels"
          filter={isLine}
          paint={{
            "line-width": 5,
            "line-color": colors.OsmRoadEdge,
          }}
          layout={{
            visibility: !$controls.showSimplified ? "visible" : "none",
          }}
        />

        <CircleLayer
          id="original-intersections"
          filter={isPoint}
          paint={{
            "circle-color": colors.OsmIntersection,
            "circle-radius": 7,
          }}
          layout={{
            visibility: !$controls.showSimplified ? "visible" : "none",
          }}
        />
      </GeoJSON>
    {/if}

    <DebuggerLayer name="face" data={debuggedFace} />
    <DebuggerLayer name="edge" data={debuggedEdge} />
  </div>
</SplitComponent>
