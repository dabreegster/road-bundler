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

  let ways: FeatureCollection<LineString> = JSON.parse($backend!.getWays());
  let faces: FeatureCollection<Polygon> = JSON.parse($backend!.getFaces());
</script>

<SplitComponent>
  <div slot="sidebar">Controls</div>

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
      />
    </GeoJSON>

    <GeoJSON data={ways}>
      <LineLayer
        id="ways"
        beforeId="Road labels"
        manageHoverState
        eventsIfTopMost
        paint={{
          "line-width": hoverStateFilter(5, 8),
          "line-color": "black",
        }}
      >
        <Popup openOn="hover" let:props>
          <h4>Way {props.osm_way}</h4>
          <PropertiesTable properties={JSON.parse(props.osm_tags)} />
        </Popup>
      </LineLayer>
    </GeoJSON>
  </div>
</SplitComponent>
