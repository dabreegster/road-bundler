<script lang="ts">
  import {
    colors,
    controls,
    tool,
    backend,
    type OriginalGraph,
    type EdgeProps,
  } from "../";
  import {
    hoverStateFilter,
    GeoJSON,
    LineLayer,
    type LayerClickInfo,
  } from "svelte-maplibre";
  import type { Feature, FeatureCollection, LineString } from "geojson";
  import { Popup } from "svelte-utils/map";
  import { constructMatchExpression, emptyGeojson } from "svelte-utils/map";
  import ListEdges from "./ListEdges.svelte";

  export let edges: FeatureCollection<LineString, EdgeProps>;
  export let afterMutation: (undoDiff: number) => void;
  export let originalGraph: OriginalGraph;
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
    originalGraph: OriginalGraph,
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

  function debugConstituentEdges(
    tmpHoveredEdge: Feature | null,
  ): FeatureCollection {
    let gj: FeatureCollection = {
      type: "FeatureCollection" as const,
      features: [],
    };
    if (tmpHoveredEdge && $tool != "width") {
      let kind = JSON.parse(tmpHoveredEdge.properties!.kind);
      if (kind.Motorized) {
        for (let e of kind.Motorized.roads) {
          let edge = JSON.parse(JSON.stringify(originalEdges[e]));
          edge.properties.simple_kind = "road";
          gj.features.push(edge);
        }
        for (let e of kind.Motorized.service_roads) {
          let edge = JSON.parse(JSON.stringify(originalEdges[e]));
          edge.properties.simple_kind = "service road";
          gj.features.push(edge);
        }
        for (let e of kind.Motorized.sidepaths) {
          let edge = JSON.parse(JSON.stringify(originalEdges[e]));
          edge.properties.simple_kind = "sidepath";
          gj.features.push(edge);
        }
        for (let e of kind.Motorized.connectors) {
          let edge = JSON.parse(JSON.stringify(originalEdges[e]));
          edge.properties.simple_kind = "connector";
          gj.features.push(edge);
        }
      } else {
        for (let e of kind.Nonmotorized) {
          let edge = JSON.parse(JSON.stringify(originalEdges[e]));
          edge.properties.simple_kind = "nonmotorized";
          gj.features.push(edge);
        }
      }
    }
    return gj;
  }
</script>

<GeoJSON data={debugConstituentEdges(tmpHoveredEdge)}>
  <LineLayer
    id="debug-constituent-edges"
    beforeId="Road labels"
    paint={{
      "line-width": 5,
      "line-color": constructMatchExpression(
        ["get", "simple_kind"],
        colors.edges,
        "red",
      ),
      "line-dasharray": [2, 2],
    }}
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
      "line-width": hoverStateFilter(5, 8),
      "line-color": constructMatchExpression(
        ["get", "simple_kind"],
        colors.edges,
        "red",
      ),
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
      {@const kind = JSON.parse(props.kind)}
      {#if kind.Motorized}
        <h4>Edge {props.edge_id}, motorized</h4>

        <ListEdges
          collection="Roads"
          edges={kind.Motorized.roads}
          {originalEdges}
        />
        <ListEdges
          collection="Service roads"
          edges={kind.Motorized.service_roads}
          {originalEdges}
        />
        <ListEdges
          collection="Sidepaths"
          edges={kind.Motorized.sidepaths}
          {originalEdges}
        />
        <ListEdges
          collection="Connectors"
          edges={kind.Motorized.connectors}
          {originalEdges}
        />
      {:else}
        <h4>Edge {props.edge_id}, non-motorized</h4>

        <ListEdges
          collection="Pieces"
          edges={kind.Nonmotorized}
          {originalEdges}
        />
      {/if}

      <p>Length {props.length}m</p>
      <p>
        Bearing {props.bearing}
        <span style:display="inline-block" style:rotate={`${props.bearing}deg`}>
          ⬆
        </span>
      </p>
    </Popup>
  </LineLayer>
</GeoJSON>
