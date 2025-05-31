<script lang="ts">
  import type { Tool } from "./";

  export let tool: Tool;

  let tools: [Tool, string, string][] = [
    ["explore", "Explore the map", "fa-solid fa-up-down-left-right"],
    ["collapseToCentroid", "Roundabouts", "fa-regular fa-circle"],
    ["dualCarriageway", "Dual carriageways", "fa-solid fa-grip-lines-vertical"],
    ["sidepath", "Sidepaths", "fa-solid fa-lines-leaning"],
    ["edge", "Edges", "fa-solid fa-dog"],
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
      tool = "edge";
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
  }
</style>
