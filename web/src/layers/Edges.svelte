<script lang="ts">
  import { colors, controls, tool, backend, type EdgeProps } from "../";
  import {
    hoverStateFilter,
    GeoJSON,
    LineLayer,
    type LayerClickInfo,
  } from "svelte-maplibre";
  import type { Feature, FeatureCollection, LineString } from "geojson";
  import { Popup } from "svelte-utils/map";
  import { PropertiesTable } from "svelte-utils";
  import { emptyGeojson } from "svelte-utils/map";

  export let edges: FeatureCollection<LineString, EdgeProps>;
  export let afterMutation: (undoDiff: number) => void;
  export let originalGraph: FeatureCollection;
  export let debuggedEdge: FeatureCollection;

  let tmpHoveredEdge: Feature | null = null;
  let originalEdges = getOriginalEdges(originalGraph);

  $: debuggedEdge = debugEdge(tmpHoveredEdge, $tool);
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

  function getOriginalEdges(
    originalGraph: FeatureCollection,
  ): Record<number, Feature> {
    let edges: Record<number, Feature> = {};
    for (let f of originalGraph.features) {
      if (f.geometry.type == "LineString") {
        edges[f.properties!.edge_id as number] = f;
      }
    }
    return edges;
  }

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
</script>

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
        Original edges: {JSON.parse(props.associated_original_edges).join(", ")}
      </p>
      <p>Length {props.length}m</p>
      <p>
        Bearing {props.bearing}
        <span style:display="inline-block" style:rotate={`${props.bearing}deg`}>
          ⬆
        </span>
      </p>

      {#if props.provenance != "Synthetic"}
        <PropertiesTable properties={JSON.parse(props.provenance).OSM.tags} />
      {/if}
    </Popup>
  </LineLayer>
</GeoJSON>

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
