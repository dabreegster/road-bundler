<script lang="ts">
  import { backend } from "./";
  import { SplitComponent } from "svelte-utils/two_column_layout";
  import {
    GeoJSON,
    LineLayer,
    hoverStateFilter,
    FillLayer,
  } from "svelte-maplibre";
  import type { LineString, FeatureCollection, Polygon } from "geojson";
  import { Popup } from "svelte-utils/map";
  import { PropertiesTable } from "svelte-utils";

  let edges: FeatureCollection<LineString> = JSON.parse($backend!.getEdges());
  let faces: FeatureCollection<Polygon, { edges: number[] }> = JSON.parse(
    $backend!.getFaces(),
  );

  let hoveredFace: Feature<Polygon, { edges: number[] }> | null = null;
  $: highlightEdges = hoveredFace
    ? JSON.parse(hoveredFace.properties.edges)
    : [];
</script>

<SplitComponent>
  <div slot="sidebar">
    {#if hoveredFace}
      {highlightEdges.length} edges touch this face: {highlightEdges}
    {/if}
  </div>

  <div slot="map">
    <GeoJSON data={faces} generateId>
      <FillLayer
        id="faces"
        beforeId="Road labels"
        manageHoverState
        eventsIfTopMost
        paint={{
          "fill-color": "cyan",
          "fill-opacity": hoverStateFilter(0.2, 1),
        }}
        bind:hovered={hoveredFace}
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
            ["in", ["id"], ["literal", highlightEdges]],
            "red",
            "black",
          ],
        }}
      >
        <Popup openOn="hover" let:props>
          <h4>Edge {props.edge_id}, Way {props.osm_way}</h4>
          <PropertiesTable properties={JSON.parse(props.osm_tags)} />
        </Popup>
      </LineLayer>
    </GeoJSON>
  </div>
</SplitComponent>
