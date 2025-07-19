<script lang="ts">
  import Sidebar from "./sidebar/Sidebar.svelte";
  import DebuggerLayer from "./layers/DebuggerLayer.svelte";
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
    type OriginalGraph,
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
    constructMatchExpression,
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
  let originalGraph: OriginalGraph = JSON.parse(
    $backend!.getOriginalOsmGraph(),
  );

  let allRoadWidths: FeatureCollection = emptyGeojson();

  let undoCount = 0;

  let hoveredFace: Feature<Polygon, FaceProps> | null = null;
  let debuggedFace = emptyGeojson();
  let debuggedEdge = emptyGeojson();

  function afterMutation(undoDiff: number) {
    edges = JSON.parse($backend!.getEdges());
    intersections = JSON.parse($backend!.getIntersections());
    faces = JSON.parse($backend!.getFaces());
    hoveredFace = null;
    undoCount += undoDiff;
  }
</script>

<SplitComponent>
  <div slot="sidebar">
    <Sidebar {undoCount} {afterMutation} bind:allRoadWidths {hoveredFace} />
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
            "line-color": constructMatchExpression(
              ["get", "simple_kind"],
              colors.edges,
              "red",
            ),
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
