<script lang="ts">
  import { colors, controls, tool, backend, type IntersectionProps } from "./";
  import { GeoJSON, CircleLayer, type LayerClickInfo } from "svelte-maplibre";
  import type { FeatureCollection, Point } from "geojson";

  export let intersections: FeatureCollection<Point, IntersectionProps>;
  export let afterMutation: (undoDiff: number) => void;

  function clickIntersection(e: CustomEvent<LayerClickInfo>) {
    try {
      let f = e.detail.features[0];
      if ($tool == "clean") {
        $backend!.collapseDegenerateIntersection(f.properties!.intersection_id);
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
      "circle-opacity": $controls.showSimplified ? 1 : 0.5,
    }}
    layout={{
      visibility: $controls.showIntersections ? "visible" : "none",
    }}
    hoverCursor={$tool == "clean" ? "pointer" : undefined}
    on:click={clickIntersection}
  />
</GeoJSON>
