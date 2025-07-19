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
  import { PropertiesTable } from "svelte-utils";
  import { constructMatchExpression, emptyGeojson } from "svelte-utils/map";

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
        <p>Roads: {kind.Motorized.roads.join(", ")}</p>
        <p>Service roads: {kind.Motorized.service_roads.join(", ")}</p>
        <p>Sidepaths: {kind.Motorized.sidepaths.join(", ")}</p>
        <p>Connectors: {kind.Motorized.connectors.join(", ")}</p>

        <!-- TODO nicer scroller -->
        {#each [...kind.Motorized.roads, ...kind.Motorized.service_roads, ...kind.Motorized.sidepaths, ...kind.Motorized.connectors] as e}
          {@const orig = originalEdges[e].properties}
          <details>
            <summary>Original edge {e}</summary>
          <a
            href={`https://www.openstreetmap.org/way/${orig.way}`}
            target="_blank"
          >
            Way {orig.way}
          </a>

        <PropertiesTable
          properties={orig.tags}
        />
          </details>
        {/each}
      {:else}
        <h4>Edge {props.edge_id}, non-motorized</h4>
        <p>Pieces: {kind.Nonmotorized.join(", ")}</p>

        <!-- TODO nicer scroller -->
        {#each kind.Nonmotorized as e }
          {@const orig = originalEdges[e].properties}
          <details>
            <summary>Original edge {e}</summary>
          <a
            href={`https://www.openstreetmap.org/way/${orig.way}`}
            target="_blank"
          >
            Way {orig.way}
          </a>

        <PropertiesTable
          properties={orig.tags}
        />
          </details>
        {/each}
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

<GeoJSON data={debugAssociatedEdges(tmpHoveredEdge)}>
  <LineLayer
    id="debug-original-edges"
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
