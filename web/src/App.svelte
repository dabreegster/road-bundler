<script lang="ts">
  import "bootstrap/dist/css/bootstrap.min.css";
  import "@fortawesome/fontawesome-free/css/all.min.css";
  import favicon from "../assets/favicon.ico?url";
  import { MapLibre } from "svelte-maplibre";
  import { PolygonToolLayer } from "maplibre-draw-polygon";
  import { onMount } from "svelte";
  import { backend } from "./";
  import type { Map } from "maplibre-gl";
  import {
    bbox,
    Basemaps,
    basemapStyles,
    StandardControls,
    MapContextMenu,
  } from "svelte-utils/map";
  import { OverpassSelector } from "svelte-utils/overpass";
  import { Loading } from "svelte-utils";
  import {
    mapContents,
    sidebarContents,
    Layout,
  } from "svelte-utils/two_column_layout";
  import * as backendPkg from "../../backend/pkg";
  import MainMode from "./MainMode.svelte";

  let loading = "";
  let map: Map | undefined;
  let style = basemapStyles["Maptiler Dataviz"];

  let examples: string[] = [];
  let loadExample = "";

  onMount(async () => {
    await backendPkg.default();

    try {
      let resp = await fetch("example_osm/list");
      if (resp.ok) {
        examples = await resp.json();
      }
    } catch (err) {}
  });

  $: loadFromExample(loadExample);

  async function loadFromExample(loadExample: string) {
    if (loadExample.length == 0) {
      return;
    }
    try {
      loading = "Loading from example file";
      let resp = await fetch(`example_osm/${loadExample}`);
      let bytes = await resp.arrayBuffer();
      $backend = new backendPkg.RoadBundler(new Uint8Array(bytes));
      zoomFit();
    } catch (err) {
      window.alert(`Bad input file: ${err}`);
    } finally {
      loading = "";
    }
  }

  let fileInput: HTMLInputElement;
  async function loadFile(e: Event) {
    try {
      loading = "Loading from file";
      let bytes = await fileInput.files![0].arrayBuffer();
      $backend = new backendPkg.RoadBundler(new Uint8Array(bytes));
      zoomFit();
    } catch (err) {
      window.alert(`Bad input file: ${err}`);
    } finally {
      loading = "";
    }
  }

  async function gotXml(e: CustomEvent<{ xml: string }>) {
    try {
      let bytes = new TextEncoder().encode(e.detail.xml);
      $backend = new backendPkg.RoadBundler(new Uint8Array(bytes));
      zoomFit();
    } catch (err) {
      window.alert(`Couldn't import from Overpass: ${err}`);
    } finally {
      loading = "";
    }
  }

  function zoomFit() {
    map!.fitBounds(bbox(JSON.parse($backend!.getEdges())), {
      animate: false,
      padding: 10,
    });
  }

  let sidebarDiv: HTMLDivElement;
  let mapDiv: HTMLDivElement;
  $: if (sidebarDiv && $sidebarContents) {
    sidebarDiv.innerHTML = "";
    sidebarDiv.appendChild($sidebarContents);
  }
  $: if (mapDiv && $mapContents) {
    mapDiv.innerHTML = "";
    mapDiv.appendChild($mapContents);
  }
</script>

<svelte:head>
  <link rel="icon" type="image/x-icon" href={favicon} />
</svelte:head>

<Loading {loading} />

<Layout>
  <div slot="left">
    <h1>Road bundler</h1>

    {#if $backend}
      <div>
        <button class="btn btn-secondary" on:click={() => ($backend = null)}>
          Load another area
        </button>
      </div>

      <br />
    {:else if map}
      {#if examples.length}
        <div>
          <label>
            Load an example
            <select class="form-select" bind:value={loadExample}>
              {#each examples as x}
                <option value={x}>{x}</option>
              {/each}
            </select>
          </label>
        </div>

        <p class="fst-italic my-3">or...</p>
      {/if}

      <div>
        <label class="form-label">
          Load an osm.pbf or osm.xml file
          <input
            class="form-control"
            bind:this={fileInput}
            on:change={loadFile}
            type="file"
          />
        </label>
      </div>

      <p class="fst-italic my-3">or...</p>

      <OverpassSelector
        {map}
        on:gotXml={gotXml}
        on:loading={(e) => (loading = e.detail)}
        on:error={(e) => window.alert(e.detail)}
      />
    {/if}

    <div bind:this={sidebarDiv} />
  </div>

  <div slot="main" style="position:relative; width: 100%; height: 100vh;">
    <MapLibre
      {style}
      hash
      bind:map
      on:error={(e) => {
        // @ts-ignore ErrorEvent isn't exported
        console.log(e.detail.error);
      }}
    >
      <StandardControls {map} />
      <MapContextMenu {map} />
      <Basemaps bind:style choice="Maptiler Dataviz" />

      {#if $backend && map}
        <div bind:this={mapDiv} />

        <MainMode />
      {:else}
        <PolygonToolLayer />
      {/if}
    </MapLibre>
  </div>
</Layout>
