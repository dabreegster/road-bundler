import { type Writable, writable } from "svelte/store";
import { RoadBundler } from "backend";

export let backend: Writable<RoadBundler | null> = writable(null);

export type Tool =
  | "explore"
  | "collapseToCentroid"
  | "dualCarriageway"
  | "sidepath"
  | "dogleg"
  | "clean"
  | "width";

export let colors = {
  UrbanBlock: "purple",
  SidepathArtifact: "yellow",
  DualCarriageway: "blue",
  RoadArtifact: "cyan",
  OtherArea: "green",

  OsmRoadEdge: "black",
  OsmSidepathEdge: "grey",
  OsmIntersection: "green",
  SyntheticEdge: "orange",
  SyntheticIntersection: "pink",

  BuildingCentroid: "black",
};
