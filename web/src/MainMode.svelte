<script lang="ts">
  import { backend } from "./";
  import { SplitComponent } from "svelte-utils/two_column_layout";
  import {
    GeoJSON,
    LineLayer,
    hoverStateFilter,
    CircleLayer,
    FillLayer,
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
    boundary_edges: number[];
    connecting_edges: number[];
    boundary_intersections: Feature<Point>[];
    num_buildings: number;
  }

  let edges: FeatureCollection<LineString> = JSON.parse($backend!.getEdges());
  let faces: FeatureCollection<Polygon, FaceProps> = JSON.parse(
    $backend!.getFaces(),
  );
  let buildings: FeatureCollection<Point> = JSON.parse(
    $backend!.getBuildings(),
  );

  let showRealBlocks = false;
  let showEdges = true;
  let showBuildings = false;

  let hoveredFace: Feature<Polygon, FaceProps> | null = null;

  $: highlightBoundaryEdges = hoveredFace
    ? JSON.parse(hoveredFace.properties.boundary_edges)
    : [];

  $: highlightBoundaryIntersections = {
    type: "FeatureCollection" as const,
    features: hoveredFace
      ? JSON.parse(hoveredFace.properties.boundary_intersections)
      : [],
  };

  $: highlightConnectingEdges = hoveredFace
    ? JSON.parse(hoveredFace.properties.connecting_edges)
    : [];
</script>

<SplitComponent>
  <div slot="sidebar">
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
      <p>{highlightBoundaryEdges.length} edges touch this face</p>
      {#each highlightBoundaryEdges as e}
        <p>{edges.features[e].properties.osm_tags.highway}</p>
      {/each}
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
        bind:hovered={hoveredFace}
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

    <GeoJSON data={edges}>
      <LineLayer
        id="edges"
        beforeId="Road labels"
        manageHoverState
        eventsIfTopMost
        paint={{
          "line-width": hoverStateFilter(5, 8),
          "line-color": [
            "case",
            ["in", ["id"], ["literal", highlightBoundaryEdges]],
            "red",
            [
              "case",
              ["in", ["id"], ["literal", highlightConnectingEdges]],
              "yellow",
              "black",
            ],
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
  </div>
</SplitComponent>
