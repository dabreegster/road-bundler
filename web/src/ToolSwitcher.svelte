<script lang="ts">
  import type { Tool } from "./";

  export let tool: Tool;

  let tools: [Tool, string, string][] = [
    ["explore", "Explore the map", "fa-solid fa-up-down-left-right"],
    ["collapseToCentroid", "Roundabouts", "fa-regular fa-circle"],
    ["dualCarriageway", "Dual carriageways", "fa-solid fa-grip-lines-vertical"],
    ["sidepath", "Sidepaths", "fa-solid fa-lines-leaning"],
    ["dogleg", "Dog-leg", "fa-solid fa-dog"],
    ["clean", "Cleanup", "fa-solid fa-broom"],
    ["width", "Road width", "fa-solid fa-ruler-horizontal"],
  ];

  function keyDown(e: KeyboardEvent) {
    if (e.key == "1") {
      tool = "explore";
    } else if (e.key == "2") {
      tool = "collapseToCentroid";
    } else if (e.key == "3") {
      tool = "dualCarriageway";
    } else if (e.key == "4") {
      tool = "sidepath";
    } else if (e.key == "5") {
      tool = "dogleg";
    } else if (e.key == "6") {
      tool = "clean";
    } else if (e.key == "7") {
      tool = "width";
    }
  }
</script>

<svelte:window on:keydown={keyDown} />

<div>
  {#each tools as [value, label, icon]}
    <button
      on:click={() => (tool = value)}
      disabled={tool == value}
      title={label}
    >
      <i class={icon}></i>
    </button>
  {/each}
</div>

<style>
  div {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
</style>
