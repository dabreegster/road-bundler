import { type Writable, writable } from "svelte/store";
import { RoadBundler } from "backend";

export let backend: Writable<RoadBundler | null> = writable(null);

export type Tool =
  | "explore"
  | "collapseToCentroid"
  | "dualCarriageway"
  | "sidepath"
  // TODO Starting to abuse this one...
  | "edge"
  | "width";

export let colors = {
  UrbanBlock: "purple",
  SidepathArtifact: "yellow",
  DualCarriageway: "blue",
  RoadArtifact: "cyan",

  OsmRoadEdge: "black",
  OsmSidepathEdge: "grey",
  OsmIntersection: "green",
  SyntheticEdge: "orange",
  SyntheticIntersection: "pink",

  BuildingCentroid: "black",
};
