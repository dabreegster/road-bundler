<script lang="ts">
  import { backend } from "./";
  import { SplitComponent } from "svelte-utils/two_column_layout";
  import { GeoJSON, LineLayer, hoverStateFilter } from "svelte-maplibre";
  import type { LineString, FeatureCollection } from "geojson";
  import { Popup } from "svelte-utils/map";
  import { PropertiesTable } from "svelte-utils";

  let ways: FeatureCollection<LineString> = JSON.parse($backend!.getWays());
</script>

<SplitComponent>
  <div slot="sidebar">Controls</div>

  <div slot="map">
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
