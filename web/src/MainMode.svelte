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
  import {
    GeoJSON,
    LineLayer,
    hoverStateFilter,
    CircleLayer,
    Control,
    type LayerClickInfo,
  } from "svelte-maplibre";
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
  import { PropertiesTable } from "svelte-utils";
  import MapPanel from "./MapPanel.svelte";
  import Faces from "./Faces.svelte";
  import Intersections from "./Intersections.svelte";

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
  let originalEdges = getOriginalEdges();

  let allRoadWidths: FeatureCollection = emptyGeojson();

  let undoCount = 0;

  let hoveredFace: Feature<Polygon, FaceProps> | null = null;
  let debuggedFace = emptyGeojson();

  let tmpHoveredEdge: Feature | null = null;

  function clickEdge(e: CustomEvent<LayerClickInfo>) {
    try {
      let f = e.detail.features[0];
      if ($tool == "dogleg") {
        $backend!.collapseEdge(f.properties!.edge_id);
      } else if ($tool == "clean") {
        $backend!.removeEdge(f.properties!.edge_id);
      } else {
        return;
      }

      afterMutation(1);
    } catch (err) {
      window.alert(
        `You probably have to refresh the app now; something broke: ${err}`,
      );
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

  function debugEdge(
    tmpHoveredEdge: Feature | null,
    tool: string,
  ): FeatureCollection {
    if (!tmpHoveredEdge || tool != "width") {
      return emptyGeojson();
    }
    return JSON.parse(
      $backend!.debugRoadWidth(tmpHoveredEdge.properties!.edge_id),
    );
  }
  $: debuggedEdge = debugEdge(tmpHoveredEdge, $tool);

  function getOriginalEdges(): Record<number, Feature> {
    let edges: Record<number, Feature> = {};
    for (let f of originalGraph.features) {
      if (f.geometry.type == "LineString") {
        edges[f.properties!.edge_id as number] = f;
      }
    }
    return edges;
  }

  function debugAssociatedEdges(
    tmpHoveredEdge: Feature | null,
  ): FeatureCollection {
    let gj: FeatureCollection = {
      type: "FeatureCollection" as const,
      features: [],
    };
    if (tmpHoveredEdge && $tool != "width") {
      for (let e of JSON.parse(
        tmpHoveredEdge.properties!.associated_original_edges,
      )) {
        let edge = originalEdges[e];
        // TODO We're not being careful to preserve original edges only
        if (edge) {
          gj.features.push(edge);
        }
      }
    }
    return gj;
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

    <GeoJSON data={edges} generateId>
      <LineLayer
        id="edges"
        beforeId="Road labels"
        manageHoverState
        bind:hovered={tmpHoveredEdge}
        eventsIfTopMost
        paint={{
          "line-width": [
            "case",
            ["==", 0, ["length", ["get", "associated_original_edges"]]],
            hoverStateFilter(5, 8),
            hoverStateFilter(7, 10),
          ],
          "line-color": [
            "case",
            ["==", ["get", "provenance"], "Synthetic"],
            colors.SyntheticEdge,
            ["get", "is_road"],
            colors.OsmRoadEdge,
            colors.OsmSidepathEdge,
          ],
          "line-opacity": $controls.showSimplified ? 1 : 0.5,
        }}
        layout={{ visibility: $controls.showEdges ? "visible" : "none" }}
        hoverCursor="pointer"
        on:click={clickEdge}
      >
        <Popup
          openOn={$tool == "dogleg" || $tool == "clean" ? "hover" : "click"}
          let:props
        >
          {#if props.provenance == "Synthetic"}
            <h4>Edge {props.edge_id}, synthetic</h4>
          {:else}
            {@const x = JSON.parse(props.provenance)}
            <h4>
              Edge {props.edge_id},
              <a
                href={`https://www.openstreetmap.org/way/${x.OSM.way}`}
                target="_blank"
              >
                Way {x.OSM.way}
              </a>
            </h4>
          {/if}

          <p>
            Original edges: {JSON.parse(props.associated_original_edges).join(
              ", ",
            )}
          </p>
          <p>Length {props.length}m</p>
          <p>
            Bearing {props.bearing}
            <span
              style:display="inline-block"
              style:rotate={`${props.bearing}deg`}
            >
              ⬆
            </span>
          </p>

          {#if props.provenance != "Synthetic"}
            <PropertiesTable
              properties={JSON.parse(props.provenance).OSM.tags}
            />
          {/if}
        </Popup>
      </LineLayer>
    </GeoJSON>

    <Intersections {intersections} {afterMutation} />

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

    <GeoJSON data={debugAssociatedEdges(tmpHoveredEdge)}>
      <LineLayer
        id="debug-original-edges"
        beforeId="Road labels"
        paint={{
          "line-width": 5,
          "line-color": colors.OsmRoadEdge,
          "line-dasharray": [2, 2],
        }}
      />
    </GeoJSON>

    <DebuggerLayer name="face" data={debuggedFace} />
    <DebuggerLayer name="edge" data={debuggedEdge} />
  </div>
</SplitComponent>
