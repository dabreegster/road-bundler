<script lang="ts">
  import { backend } from "./";
  import { SplitComponent } from "svelte-utils/two_column_layout";
  import {
    GeoJSON,
    LineLayer,
    hoverStateFilter,
    CircleLayer,
    FillLayer,
    type LayerClickInfo,
  } from "svelte-maplibre";
  import type {
    LineString,
    Feature,
    FeatureCollection,
    Polygon,
    Point,
  } from "geojson";
  import { Popup } from "svelte-utils/map";
  import { PropertiesTable } from "svelte-utils";

  interface FaceProps {
    face_id: number;
    boundary_edges: number[];
    connecting_edges: number[];
    boundary_intersections: Feature<Point>[];
    num_buildings: number;

    dual_carriageway?: {
      name: string;
      bearings: number[];
      side1_edges: number[];
      side2_edges: number[];
      side1: Feature<LineString>;
      side2: Feature<LineString>;
    };
  }

  interface EdgeProps {
    edge_id: number;
    osm_way: number;
    osm_tags: Record<string, string>;
  }

  let edges: FeatureCollection<LineString, EdgeProps> = JSON.parse(
    $backend!.getEdges(),
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
  let showBuildings = false;

  let tmpHoveredFace: Feature | null = null;
  // Maplibre breaks nested properties
  $: hoveredFace = tmpHoveredFace
    ? faces.features[tmpHoveredFace.id! as number]
    : (null as Feature<Polygon, FaceProps> | null);

  $: highlightBoundaryEdges = hoveredFace?.properties.boundary_edges || [];

  $: highlightBoundaryIntersections = {
    type: "FeatureCollection" as const,
    features: hoveredFace?.properties.boundary_intersections || [],
  };

  $: highlightConnectingEdges =
    tool != "dualCarriageway"
      ? hoveredFace?.properties.connecting_edges || []
      : [];

  $: highlightSide1Edges =
    tool == "dualCarriageway"
      ? hoveredFace?.properties.dual_carriageway?.side1_edges || []
      : [];
  $: highlightSide2Edges =
    tool == "dualCarriageway"
      ? hoveredFace?.properties.dual_carriageway?.side2_edges || []
      : [];

  function lookupEdge(id: number): Feature<LineString, EdgeProps> {
    for (let f of edges.features) {
      if (f.properties.edge_id == id) {
        return f;
      }
    }
    throw new Error(`edge ${id} doesn't exist`);
  }

  function collapseFace(e: CustomEvent<LayerClickInfo>) {
    if (tool != "collapseToCentroid") {
      return;
    }

    $backend!.collapseToCentroid(e.detail.features[0].properties!.face_id);

    edges = JSON.parse($backend!.getEdges());
    faces = JSON.parse($backend!.getFaces());
    hoveredFace = null;
    undoCount = undoCount + 1;
  }

  function undo() {
    $backend!.undo();

    edges = JSON.parse($backend!.getEdges());
    faces = JSON.parse($backend!.getFaces());
    hoveredFace = null;
    undoCount = undoCount - 1;
  }

  function keyDown(e: KeyboardEvent) {
    if (e.key == "z" && e.ctrlKey && undoCount > 0) {
      e.stopPropagation();
      undo();
    }
  }

  function showDCSides(
    hoveredFace: Feature<Polygon, FaceProps> | null,
    tool: string,
  ): FeatureCollection {
    let features = [] as Feature[];
    if (
      tool == "dualCarriageway" &&
      hoveredFace &&
      hoveredFace.properties.dual_carriageway
    ) {
      let dc = hoveredFace.properties.dual_carriageway;
      dc.side1.properties = { side: "A" };
      dc.side2.properties = { side: "B" };
      features = [dc.side1, dc.side2];
    }
    return {
      type: "FeatureCollection" as const,
      features,
    };
  }
</script>

<svelte:window on:keydown={keyDown} />

<SplitComponent>
  <div slot="sidebar">
    <select bind:value={tool}>
      <option value="explore">Just pan around the map</option>
      <option value="collapseToCentroid">
        Click to collapse a face to its centroid
      </option>
      <option value="dualCarriageway">Hover to debug dual carriageways</option>
    </select>

    <button class="secondary" on:click={undo} disabled={undoCount == 0}>
      Undo ({undoCount})
    </button>

    <label>
      <input type="checkbox" bind:checked={showRealBlocks} />
      Show real blocks
    </label>

    <label>
      <input type="checkbox" bind:checked={showEdges} />
      Show edges
    </label>

    <label>
      <input type="checkbox" bind:checked={showBuildings} />
      Show building centroids
    </label>

    {#if hoveredFace}
      {#if tool == "explore" || tool == "collapseToCentroid"}
        <p>{highlightBoundaryEdges.length} edges touch this face</p>
        {#each highlightBoundaryEdges as e}
          <p>{lookupEdge(e).properties.osm_tags.highway}</p>
        {/each}
      {:else if tool == "dualCarriageway"}
        {#if hoveredFace.properties.dual_carriageway}
          {@const dc = hoveredFace.properties.dual_carriageway}
          <p>{dc.name}</p>
          <p>{dc.bearings.map((b) => Math.round(b)).join(", ")}</p>
        {:else}
          <p>Not a dual carriageway</p>
        {/if}
      {/if}
    {/if}
  </div>

  <div slot="map">
    <GeoJSON data={faces} generateId>
      <FillLayer
        id="faces"
        beforeId="Road labels"
        manageHoverState
        eventsIfTopMost
        filter={showRealBlocks
          ? undefined
          : ["==", ["get", "num_buildings"], 0]}
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
        on:click={collapseFace}
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
            ["in", ["get", "edge_id"], ["literal", highlightSide1Edges]],
            "purple",
            ["in", ["get", "edge_id"], ["literal", highlightSide2Edges]],
            "blue",
            ["in", ["get", "edge_id"], ["literal", highlightBoundaryEdges]],
            "red",
            ["in", ["get", "edge_id"], ["literal", highlightConnectingEdges]],
            "yellow",
            "black",
          ],
        }}
        layout={{ visibility: showEdges ? "visible" : "none" }}
      >
        <Popup openOn="hover" let:props>
          <h4>Edge {props.edge_id}, Way {props.osm_way}</h4>
          <PropertiesTable properties={JSON.parse(props.osm_tags)} />
        </Popup>
      </LineLayer>
    </GeoJSON>

    <GeoJSON data={highlightBoundaryIntersections}>
      <CircleLayer
        id="boundary-intersections"
        paint={{
          "circle-color": "green",
          "circle-radius": 3,
        }}
      />
    </GeoJSON>

    <GeoJSON data={showDCSides(hoveredFace, tool)}>
      <LineLayer
        id="dual-carriageway-sides"
        beforeId="Road labels"
        paint={{
          "line-width": 15,
          "line-opacity": 0.5,
          "line-color": [
            "case",
            ["==", ["get", "side"], "A"],
            "purple",
            "blue",
          ],
        }}
      />
    </GeoJSON>
  </div>
</SplitComponent>
