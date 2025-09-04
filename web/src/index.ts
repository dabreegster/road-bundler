import { type Writable, writable } from "svelte/store";
import * as backendPkg from "../../backend/pkg";
import type { FeatureCollection } from "geojson";

export let backend: Writable<backendPkg.RoadBundler | null> = writable(null);

export let controls: Writable<{
  showFaces: boolean;
  showUrbanBlocks: boolean;
  showEdges: boolean;
  showIntersections: boolean;
  showBuildings: boolean;
  showSimplified: boolean;
}> = writable({
  showFaces: true,
  showUrbanBlocks: false,
  showEdges: true,
  showIntersections: false,
  showBuildings: false,
  showSimplified: true,
});

export let tool: Writable<
  | "explore"
  | "collapseToCentroid"
  | "dualCarriageway"
  | "sidepath"
  | "dogleg"
  | "clean"
  | "width"
  | "sidewalker"
> = writable("explore");

export interface FaceProps {
  face_id: number;
  debug_hover: FeatureCollection;
  kind: "UrbanBlock" | "RoadArtifact" | "SidepathArtifact" | "OtherArea";

  dual_carriageway:
    | {
        name: string;
        debug_hover: FeatureCollection;
      }
    | string;
  sidepath: FeatureCollection | string;
  generated_sidewalks: FeatureCollection;
}

export interface EdgeProps {
  edge_id: number;
  simple_kind:
    | "road"
    | "service road"
    | "sidepath"
    | "connector"
    | "nonmotorized";
  kind:
    | {
        Nonmotorized: number[];
      }
    | {
        Motorized: {
          roads: number[];
          service_roads: number[];
          sidepaths: number[];
          connectors: number[];
        };
      };
  length: number;
  bearing: number;
}

export interface IntersectionProps {
  intersection_id: number;
}

// TODO Lost some of the specifity here, boo
export type OriginalGraph = FeatureCollection;

export let colors = {
  UrbanBlock: "purple",
  SidepathArtifact: "yellow",
  DualCarriageway: "blue",
  RoadArtifact: "cyan",
  OtherArea: "green",

  edges: {
    road: "black",
    "service road": "grey",
    sidepath: "green",
    connector: "#80EF80",
    nonmotorized: "orange",
  },

  Intersection: "purple",

  BuildingCentroid: "black",
};

// TODO Narrowest is barely visible
export let widthColorScale = [
  "#f1eef6",
  "#d7b5d8",
  "#df65b0",
  "#dd1c77",
  "#980043",
];
export let widthLimits = [0, 10, 20, 30, 40, 100];
