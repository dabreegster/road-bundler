import { type Writable, writable } from "svelte/store";
import { RoadBundler } from "backend";
import type { FeatureCollection } from "geojson";

export let backend: Writable<RoadBundler | null> = writable(null);

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
}

export interface EdgeProps {
  edge_id: number;
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
  provenance:
    | {
        OSM: {
          way: number;
          node1: number;
          node2: number;
        };
      }
    | "Synthetic";
  is_road: boolean;
  length: number;
  bearing: number;
  associated_original_edges: number[];
}

export interface IntersectionProps {
  intersection_id: number;
  provenance: { OSM: number } | "Synthetic";
}

export type OriginalGraph = FeatureCollection & {
  tags_per_way: Record<number, Record<string, string>>;
};

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

// TODO Narrowest is barely visible
export let widthColorScale = [
  "#f1eef6",
  "#d7b5d8",
  "#df65b0",
  "#dd1c77",
  "#980043",
];
export let widthLimits = [0, 10, 20, 30, 40, 100];
