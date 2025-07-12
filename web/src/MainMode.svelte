<script lang="ts">
  import { RoadBundler } from "backend";
  import DebuggerLayer from "./DebuggerLayer.svelte";
  import DebuggerLegend from "./DebuggerLegend.svelte";
  import ToolSwitcher from "./ToolSwitcher.svelte";
  import { colors, backend, type Tool } from "./";
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
  import type { ExpressionSpecification } from "maplibre-gl";
  import type {
    LineString,
    Feature,
    FeatureCollection,
    Polygon,
    Point,
  } from "geojson";
  import { isLine, isPoint, emptyGeojson, Popup } from "svelte-utils/map";
  import {
    PropertiesTable,
    QualitativeLegend,
    downloadGeneratedFile,
  } from "svelte-utils";

  interface FaceProps {
    face_id: number;
    debug_hover: FeatureCollection;
    kind: "UrbanBlock" | "RoadArtifact" | "SidepathArtifact";

    dual_carriageway:
      | {
          name: string;
          debug_hover: FeatureCollection;
        }
      | string;
    sidepath: FeatureCollection | string;
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
    is_road: boolean;
    length: number;
    bearing: number;
    associated_original_edges: number[];
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
  let originalGraph: FeatureCollection = JSON.parse(
    $backend!.getOriginalOsmGraph(),
  );
  let originalEdges = getOriginalEdges();

  let tool: Tool = "explore";
  let undoCount = 0;

  let showFaces = true;
  let showUrbanBlocks = false;
  let showEdges = true;
  let showIntersections = false;
  let showBuildings = false;
  let showSimplified = true;

  let tmpHoveredFace: Feature | null = null;
  // Maplibre breaks nested properties
  $: hoveredFace = tmpHoveredFace
    ? faces.features[tmpHoveredFace.id! as number]
    : (null as Feature<Polygon, FaceProps> | null);

  let tmpHoveredEdge: Feature | null = null;

  function clickFace(e: CustomEvent<LayerClickInfo>) {
    try {
      let f = e.detail.features[0];
      if (tool == "collapseToCentroid") {
        $backend!.collapseToCentroid(f.properties!.face_id);
      } else if (tool == "dualCarriageway") {
        if (!f.properties!.dual_carriageway.startsWith("{")) {
          window.alert("This isn't a dual carriageway face");
          return;
        }
        $backend!.collapseDualCarriageway(f.properties!.face_id);
      } else if (tool == "sidepath") {
        if (f.properties!.kind != "SidepathArtifact") {
          window.alert("This isn't a sidepath face");
          return;
        }
        $backend!.mergeSidepath(f.properties!.face_id);
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

  function clickEdge(e: CustomEvent<LayerClickInfo>) {
    try {
      let f = e.detail.features[0];
      if (tool == "edge") {
        $backend!.collapseEdge(f.properties!.edge_id);
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

  function clickIntersection(e: CustomEvent<LayerClickInfo>) {
    try {
      let f = e.detail.features[0];
      if (tool == "edge") {
        $backend!.collapseDegenerateIntersection(f.properties!.intersection_id);
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

  function doBulkEdit(cb: (b: RoadBundler) => number) {
    try {
      let newCommands = cb($backend!);
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

    if (e.key == "s") {
      showSimplified = !showSimplified;
    }
  }

  function debugFace(
    hoveredFace: Feature<Polygon, FaceProps> | null,
    tool: string,
  ): FeatureCollection {
    if (!hoveredFace) {
      return emptyGeojson();
    }

    if (tool == "explore" || tool == "collapseToCentroid") {
      return hoveredFace.properties.debug_hover;
    }

    if (
      tool == "dualCarriageway" &&
      typeof hoveredFace.properties.dual_carriageway != "string"
    ) {
      return hoveredFace.properties.dual_carriageway.debug_hover;
    } else if (
      tool == "sidepath" &&
      typeof hoveredFace.properties.sidepath != "string"
    ) {
      return hoveredFace.properties.sidepath;
    }
    return emptyGeojson();
  }
  $: debuggedFace = debugFace(hoveredFace, tool);

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
  $: debuggedEdge = debugEdge(tmpHoveredEdge, tool);

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
    if (tmpHoveredEdge && tool != "width") {
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

  function downloadSidepathsAndRoads() {
    downloadGeneratedFile(
      "roads.geojson",
      JSON.stringify({
        type: "FeatureCollection",
        features: edges.features.filter((f) => f.properties.is_road),
      }),
    );
    downloadGeneratedFile(
      "sidepaths.geojson",
      JSON.stringify({
        type: "FeatureCollection",
        features: edges.features.filter((f) => !f.properties.is_road),
      }),
    );
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

  // TS fail
  $: faceFillColor = [
    "case",
    ["==", ["get", "kind"], "UrbanBlock"],
    colors.UrbanBlock,
    ["==", ["get", "kind"], "SidepathArtifact"],
    colors.SidepathArtifact,
    ["!=", ["typeof", ["get", "dual_carriageway"]], "string"],
    tool == "dualCarriageway" ? colors.DualCarriageway : colors.RoadArtifact,
    colors.RoadArtifact,
  ] as unknown as ExpressionSpecification;
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

    <ToolSwitcher bind:tool />

    {#if tool == "explore"}
      <p>Just pan around the map</p>
      <button class="outline" on:click={doAllSimplifications}>
        Do all simplifications
      </button>
    {:else if tool == "collapseToCentroid"}
      <p>Click to collapse a face to its centroid</p>
    {:else if tool == "dualCarriageway"}
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
    {:else if tool == "sidepath"}
      <p>Click to merge a sidepath into the road</p>

      <button
        class="outline"
        on:click={() => doBulkEdit((b) => b.fixAllSidepaths())}
      >
        Merge all sidepaths
      </button>

      <button
        class="outline"
        on:click={() => doBulkEdit((b) => b.removeAllSidepaths())}
      >
        Remove all sidepaths
      </button>

      <button class="outline" on:click={downloadSidepathsAndRoads}>
        Download GJ of sidepaths and roads
      </button>
    {:else if tool == "edge"}
      <p>Click an edge or degenerate intersection to collapse it</p>

      <button
        class="outline"
        on:click={() => doBulkEdit((b) => b.fixAllDogLegs())}
      >
        Collapse all dog-leg intersections
      </button>

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
    {:else if tool == "width"}
      <p>Hover on an edge to measure its width</p>
    {/if}
  </div>

  <div slot="map">
    <Control position="top-right">
      <div class="map-panel">
        <details open>
          <summary>Layers</summary>
          <label>
            <u>S</u>
            how original OSM
            <input
              type="checkbox"
              role="switch"
              bind:checked={showSimplified}
            />
            <u>S</u>
            how simplified graph
          </label>

          <br />

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
              "urban block": colors.UrbanBlock,
              "road artifact": colors.RoadArtifact,
              "sidepath artifact": colors.SidepathArtifact,
            }}
            itemsPerRow={3}
          />

          <label>
            <input type="checkbox" bind:checked={showFaces} />
            Show faces
          </label>

          <label>
            <input type="checkbox" bind:checked={showUrbanBlocks} />
            Show urban blocks
          </label>

          <hr />

          <QualitativeLegend
            labelColors={{
              "OSM road edge": colors.OsmRoadEdge,
              "OSM sidewalk/cycleway edge": colors.OsmSidepathEdge,
              "OSM intersection": colors.OsmIntersection,
              "synthetic edge": colors.SyntheticEdge,
              "synthetic intersection": colors.SyntheticIntersection,
            }}
            itemsPerRow={1}
          />

          <hr />

          <DebuggerLegend data={debuggedFace} />
          <DebuggerLegend data={debuggedEdge} />
        </details>
      </div>
    </Control>

    <GeoJSON data={faces} generateId>
      <FillLayer
        id="faces"
        beforeId="Road labels"
        manageHoverState
        filter={showUrbanBlocks
          ? undefined
          : ["!=", ["get", "kind"], "UrbanBlock"]}
        paint={{
          "fill-color": faceFillColor,
          "fill-opacity": hoverStateFilter(0.2, 1),
        }}
        layout={{ visibility: showFaces ? "visible" : "none" }}
        bind:hovered={tmpHoveredFace}
        hoverCursor={[
          "collapseToCentroid",
          "dualCarriageway",
          "sidepath",
        ].includes(tool)
          ? "pointer"
          : undefined}
        on:click={clickFace}
      />
    </GeoJSON>

    <GeoJSON data={buildings}>
      <CircleLayer
        id="buildings"
        paint={{
          "circle-color": colors.BuildingCentroid,
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
          "line-opacity": showSimplified ? 1 : 0.5,
        }}
        layout={{ visibility: showEdges ? "visible" : "none" }}
        hoverCursor="pointer"
        on:click={clickEdge}
      >
        <Popup openOn={tool == "edge" ? "hover" : "click"} let:props>
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

    <GeoJSON data={intersections}>
      <CircleLayer
        id="intersections"
        paint={{
          "circle-color": [
            "case",
            ["==", ["get", "provenance"], "Synthetic"],
            colors.SyntheticIntersection,
            colors.OsmIntersection,
          ],
          "circle-radius": 7,
          "circle-opacity": showSimplified ? 1 : 0.5,
        }}
        layout={{ visibility: showIntersections ? "visible" : "none" }}
        hoverCursor={tool == "edge" ? "pointer" : undefined}
        on:click={clickIntersection}
      />
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
          layout={{ visibility: !showSimplified ? "visible" : "none" }}
        />

        <CircleLayer
          id="original-intersections"
          filter={isPoint}
          paint={{
            "circle-color": colors.OsmIntersection,
            "circle-radius": 7,
          }}
          layout={{ visibility: !showSimplified ? "visible" : "none" }}
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

<style>
  .map-panel {
    background: white;
    padding: 16px;
  }
</style>
