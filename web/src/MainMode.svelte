<script lang="ts">
  import DebuggerLayer from "./DebuggerLayer.svelte";
  import DebuggerLegend from "./DebuggerLegend.svelte";
  import { backend } from "./";
  import { SplitComponent } from "svelte-utils/two_column_layout";
  import {
    GeoJSON,
    LineLayer,
    hoverStateFilter,
    CircleLayer,
    FillLayer,
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
  import { emptyGeojson, Popup } from "svelte-utils/map";
  import { PropertiesTable, QualitativeLegend } from "svelte-utils";

  interface FaceProps {
    face_id: number;
    debug_hover: FeatureCollection;
    num_buildings: number;
    has_parking_aisle: boolean;

    dual_carriageway:
      | {
          name: string;
          bearings: number[];
          debug_hover: FeatureCollection;
        }
      | string;
  }

  interface EdgeProps {
    edge_id: number;
    provenance:
      | {
          OSM: {
            way: number;
            node1: number;
            node2: number;
            tags: Record<string, string>;
          };
        }
      | "Synthetic";
  }

  interface IntersectionProps {
    intersection_id: number;
    provenance: { OSM: number } | "Synthetic";
  }

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

  let tool: "explore" | "collapseToCentroid" | "dualCarriageway" = "explore";
  let undoCount = 0;

  let showRealBlocks = false;
  let showEdges = true;
  let showIntersections = true;
  let showBuildings = false;

  let tmpHoveredFace: Feature | null = null;
  // Maplibre breaks nested properties
  $: hoveredFace = tmpHoveredFace
    ? faces.features[tmpHoveredFace.id! as number]
    : (null as Feature<Polygon, FaceProps> | null);

  function clickFace(e: CustomEvent<LayerClickInfo>) {
    try {
      if (tool == "collapseToCentroid") {
        $backend!.collapseToCentroid(e.detail.features[0].properties!.face_id);
      } else if (tool == "dualCarriageway") {
        $backend!.collapseDualCarriageway(
          e.detail.features[0].properties!.face_id,
        );
      } else {
        return;
      }

      afterMutation();
      undoCount = undoCount + 1;
    } catch (err) {
      window.alert(
        `You probably have to refresh the app now; something broke: ${err}`,
      );
    }
  }

  function undo() {
    try {
      $backend!.undo();

      afterMutation();
      undoCount = undoCount - 1;
    } catch (err) {
      window.alert(
        `You probably have to refresh the app now; something broke: ${err}`,
      );
    }
  }

  function fixAllDCs() {
    try {
      let newCommands = $backend!.fixAllDualCarriageways();

      afterMutation();
      undoCount = undoCount + newCommands;
    } catch (err) {
      window.alert(
        `You probably have to refresh the app now; something broke: ${err}`,
      );
    }
  }

  function afterMutation() {
    edges = JSON.parse($backend!.getEdges());
    intersections = JSON.parse($backend!.getIntersections());
    faces = JSON.parse($backend!.getFaces());
    hoveredFace = null;
  }

  function keyDown(e: KeyboardEvent) {
    if (e.key == "z" && e.ctrlKey && undoCount > 0) {
      e.stopPropagation();
      undo();
    }

    if (e.key == "1") {
      tool = "explore";
    } else if (e.key == "2") {
      tool = "collapseToCentroid";
    } else if (e.key == "3") {
      tool = "dualCarriageway";
    }
  }

  function debugFace(
    hoveredFace: Feature<Polygon, FaceProps> | null,
    tool: string,
  ): FeatureCollection {
    if (tool != "dualCarriageway" && hoveredFace) {
      return hoveredFace.properties.debug_hover;
    }

    if (
      tool == "dualCarriageway" &&
      hoveredFace &&
      typeof hoveredFace.properties.dual_carriageway != "string"
    ) {
      return hoveredFace.properties.dual_carriageway.debug_hover;
    }
    return emptyGeojson();
  }
</script>

<svelte:window on:keydown={keyDown} />

<SplitComponent>
  <div slot="sidebar">
    <select bind:value={tool}>
      <option value="explore">Explore the map</option>
      <option value="collapseToCentroid">Roundabouts</option>
      <option value="dualCarriageway">Dual carriageways</option>
    </select>

    <button class="secondary" on:click={undo} disabled={undoCount == 0}>
      Undo ({undoCount})
    </button>

    {#if tool == "explore"}
      <p>Just pan around the map</p>
    {:else if tool == "collapseToCentroid"}
      <p>Click to collapse a face to its centroid</p>
    {:else if tool == "dualCarriageway"}
      <p>Click to collapse a dual carriageway</p>

      <button class="secondary" on:click={fixAllDCs}>Collapse all DCs</button>

      {#if hoveredFace}
        {#if typeof hoveredFace.properties.dual_carriageway == "string"}
          <p>
            Not a dual carriageway: {hoveredFace.properties.dual_carriageway}
          </p>
        {:else}
          {@const dc = hoveredFace.properties.dual_carriageway}
          <p>{dc.name}</p>
          <p>Bearings: {dc.bearings.map((b) => Math.round(b)).join(", ")}</p>
        {/if}
      {/if}
    {/if}
  </div>

  <div slot="map">
    <Control position="top-right">
      <div class="map-panel">
        <label>
          <input type="checkbox" bind:checked={showRealBlocks} />
          Show real blocks
        </label>

        <label>
          <input type="checkbox" bind:checked={showEdges} />
          Show edges
        </label>

        <label>
          <input type="checkbox" bind:checked={showIntersections} />
          Show intersections
        </label>

        <label>
          <input type="checkbox" bind:checked={showBuildings} />
          Show building centroids
        </label>

        <hr />
        <QualitativeLegend
          labelColors={{
            "OSM edge": "black",
            "OSM intersection": "green",
            "synthetic edge": "orange",
            "synthetic intersection": "pink",
          }}
          itemsPerRow={1}
        />

        <DebuggerLegend data={debugFace(hoveredFace, tool)} />
      </div>
    </Control>

    <GeoJSON data={faces} generateId>
      <FillLayer
        id="faces"
        beforeId="Road labels"
        manageHoverState
        filter={showRealBlocks
          ? undefined
          : [
              "all",
              ["==", ["get", "num_buildings"], 0],
              ["!", ["get", "has_parking_aisle"]],
            ]}
        paint={{
          "fill-color": [
            "case",
            [">", ["get", "num_buildings"], 0],
            "purple",
            "cyan",
          ],
          "fill-opacity": hoverStateFilter(0.2, 1),
        }}
        bind:hovered={tmpHoveredFace}
        hoverCursor={tool == "explore" ? undefined : "pointer"}
        on:click={clickFace}
      />
    </GeoJSON>

    <GeoJSON data={buildings}>
      <CircleLayer
        id="buildings"
        paint={{
          "circle-color": "black",
          "circle-radius": 3,
        }}
        layout={{ visibility: showBuildings ? "visible" : "none" }}
      />
    </GeoJSON>

    <GeoJSON data={edges} generateId>
      <LineLayer
        id="edges"
        beforeId="Road labels"
        manageHoverState
        eventsIfTopMost
        paint={{
          "line-width": hoverStateFilter(5, 8),
          "line-color": [
            "case",
            ["==", ["get", "provenance"], "Synthetic"],
            "orange",
            "black",
          ],
        }}
        layout={{ visibility: showEdges ? "visible" : "none" }}
      >
        <Popup openOn="hover" let:props>
          {#if props.provenance == "Synthetic"}
            <h4>Edge {props.edge_id}, synthetic</h4>
          {:else}
            {@const x = JSON.parse(props.provenance)}
            <h4>Edge {props.edge_id}, Way {x.OSM.way}</h4>
            <PropertiesTable properties={x.OSM.tags} />
          {/if}
        </Popup>
      </LineLayer>
    </GeoJSON>

    <GeoJSON data={intersections}>
      <CircleLayer
        id="intersections"
        paint={{
          "circle-color": [
            "case",
            ["==", ["get", "provenance"], "Synthetic"],
            "pink",
            "green",
          ],
          "circle-radius": 7,
        }}
        layout={{ visibility: showIntersections ? "visible" : "none" }}
      />
    </GeoJSON>

    <DebuggerLayer data={debugFace(hoveredFace, tool)} />
  </div>
</SplitComponent>

<style>
  .map-panel {
    background: white;
    padding: 16px;
  }
</style>
