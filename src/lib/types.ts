export type Kind = "Normal" | "Notable" | "Keystone" | "Mastery" | "Jewel" | "Root";

/** What the backend publishes after every change (see `watcher::Snapshot`). */
export type Snapshot = {
  log: "searching" | "ok" | "not-found" | "error";
  log_path: string;
  state: { area_name: string; area_level: number; level: number; town: boolean; character: string | null };
  views: {
    route: {
      act: number;
      index: number;
      act_step: number;
      act_total: number;
      step: string;
      label: string;
      hints: string[];
      next: string[];
      rewards: { gem: string; how: "Take" | "Buy"; npc: string }[];
      passive_points: number;
    } | null;
    plan: {
      stage: number;
      stage_title: string;
      stages: string[];
      manual: boolean;
      auto_order: boolean;
      class: string;
      ascendancy: string;
      bandit: string;
      passives_total: number;
      passives_available: number;
      budget: number | null;
      spent_before: number;
      tree: { x: number; y: number; from: [number, number] | null; kind: Kind; state: "done" | "now" | "later" }[];
      passives: { name: string; kind: Kind; stats: string[]; known: boolean; available: boolean }[];
      masteries: [string, string[]][];
      new_gems: string[];
      gems: { name: string; support: boolean }[][];
      gear: { slot: string; name: string; base: string; changed: boolean }[];
      notes: string;
    } | null;
  };
};
