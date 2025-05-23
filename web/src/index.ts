import { type Writable, writable } from "svelte/store";
import { RoadBundler } from "backend";

export let backend: Writable<RoadBundler | null> = writable(null);
