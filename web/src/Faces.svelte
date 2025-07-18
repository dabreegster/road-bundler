<script lang="ts">
  import type { Feature, FeatureCollection, Polygon } from "geojson";
  import { backend, colors, controls, tool, type FaceProps } from "./";
  import {
    GeoJSON,
    FillLayer,
    hoverStateFilter,
    type LayerClickInfo,
  } from "svelte-maplibre";
  import type { ExpressionSpecification } from "maplibre-gl";
  import { emptyGeojson } from "svelte-utils/map";

  export let faces: FeatureCollection<Polygon, FaceProps>;
  export let hoveredFace: Feature<Polygon, FaceProps> | null;
  export let debuggedFace: FeatureCollection;
  export let afterMutation: (undoDiff: number) => void;

  let tmpHoveredFace: Feature | null = null;
  // Maplibre breaks nested properties
  $: hoveredFace = tmpHoveredFace
    ? faces.features[tmpHoveredFace.id! as number]
    : (null as Feature<Polygon, FaceProps> | null);

  $: debuggedFace = debugFace(hoveredFace, $tool);

  // TS fail
  $: faceFillColor = [
    "case",
    ["==", ["get", "kind"], "UrbanBlock"],
    colors.UrbanBlock,
    ["==", ["get", "kind"], "SidepathArtifact"],
    colors.SidepathArtifact,
    ["==", ["get", "kind"], "OtherArea"],
    colors.OtherArea,
    ["!=", ["typeof", ["get", "dual_carriageway"]], "string"],
    $tool == "dualCarriageway" ? colors.DualCarriageway : colors.RoadArtifact,
    colors.RoadArtifact,
  ] as unknown as ExpressionSpecification;

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

  function clickFace(e: CustomEvent<LayerClickInfo>) {
    try {
      let f = e.detail.features[0];
      if ($tool == "collapseToCentroid") {
        $backend!.collapseToCentroid(f.properties!.face_id);
      } else if ($tool == "dualCarriageway") {
        if (!f.properties!.dual_carriageway.startsWith("{")) {
          window.alert("This isn't a dual carriageway face");
          return;
        }
        $backend!.collapseDualCarriageway(f.properties!.face_id);
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
</script>

<GeoJSON data={faces} generateId>
  <FillLayer
    id="faces"
    beforeId="Road labels"
    manageHoverState
    filter={$controls.showUrbanBlocks
      ? undefined
      : ["!=", ["get", "kind"], "UrbanBlock"]}
    paint={{
      "fill-color": faceFillColor,
      "fill-opacity": hoverStateFilter(0.2, 1),
    }}
    layout={{ visibility: $controls.showFaces ? "visible" : "none" }}
    bind:hovered={tmpHoveredFace}
    hoverCursor={["collapseToCentroid", "dualCarriageway"].includes($tool)
      ? "pointer"
      : undefined}
    on:click={clickFace}
  />
</GeoJSON>
