//! Leveling plan: one stage per PoB spec, each with what to allocate, which gems to use and which gear to aim for.
//! Stages are matched across specs, skill sets and item sets by title (index as fallback), the way guide authors
//! name them ("1) Level 1-12", "lv 24+", "Normal Lab").

use std::collections::HashSet;

use serde::Serialize;

use crate::pob_model::{Build, GemGroup, Item};
use crate::tree::{take_order, Step, TreeData};

/// Character level at which each lab becomes doable, and at which each act roughly starts.
const LABS: [(&str, u32); 4] = [("normal lab", 33), ("cruel lab", 55), ("merc", 68), ("eternal lab", 75)];
const ACT_START: [u32; 10] = [1, 13, 23, 33, 41, 48, 55, 60, 65, 68];

/// Level a stage starts at, read from its title. None = no level in the title (switch manually).
pub fn level_from_title(title: &str) -> Option<u32> {
    let t = title.to_lowercase();
    let digits_after = |at: usize| -> Option<u32> {
        let rest = t[at..].trim_start_matches(['.', ' ']);
        let n: String = rest.chars().take_while(char::is_ascii_digit).collect();
        n.parse().ok()
    };
    // explicit level first: "level 1-12", "lvl 24+", "lv 40+ (Merc Swap)"
    let explicit = ["level", "lvl", "lv"]
        .iter()
        .filter_map(|w| t.find(w).map(|i| (i, w.len())))
        .min_by_key(|(i, _)| *i)
        .and_then(|(i, len)| digits_after(i + len));
    if explicit.is_some() {
        return explicit;
    }
    if let Some((_, lvl)) = LABS.iter().find(|(name, _)| t.contains(name)) {
        return Some(*lvl);
    }
    t.find("act").and_then(|i| digits_after(i + 3)).filter(|a| (1..=10).contains(a)).map(|a| ACT_START[a as usize - 1])
}

#[derive(Debug, Serialize)]
pub struct GearTarget {
    pub slot: String,
    pub item: Item,
    /// differs from the previous stage
    pub changed: bool,
}

#[derive(Debug, Serialize)]
pub struct Stage {
    pub title: String,
    pub from_level: Option<u32>,
    /// points to allocate on top of the previous stage, in order
    pub passives: Vec<Step>,
    /// mastery choices made in this stage: (mastery name, chosen effect stats)
    pub masteries: Vec<(String, Vec<String>)>,
    pub gems: Vec<GemGroup>,
    /// gem names that were not in the previous stage
    pub new_gems: Vec<String>,
    pub gear: Vec<GearTarget>,
}

#[derive(Debug, Serialize)]
pub struct Plan {
    pub class: String,
    pub ascendancy: String,
    pub bandit: String,
    pub pantheon_major: String,
    pub pantheon_minor: String,
    pub notes: String,
    /// true when the PoB has a single tree: the passive order is reconstructed, not authored by the guide
    pub auto_order: bool,
    pub stages: Vec<Stage>,
}

fn by_title<'a, T>(items: &'a [T], title_of: impl Fn(&T) -> &str, title: &str, index: usize) -> Option<&'a T> {
    items.iter().find(|i| title_of(i) == title).or_else(|| items.get(index))
}

pub fn build(b: &Build, tree: &TreeData) -> Plan {
    let mut stages: Vec<Stage> = Vec::new();
    for (i, spec) in b.specs.iter().enumerate() {
        let prev = i.checked_sub(1).map(|p| &b.specs[p]);
        let skills = by_title(&b.skill_sets, |s| &s.title, &spec.title, i);
        let gems: Vec<GemGroup> = skills.map(|s| s.groups.clone()).unwrap_or_default();
        let before: HashSet<&str> = stages.last().map(|s| s.gems.iter().flat_map(|g| g.gems.iter().map(|g| g.name.as_str())).collect()).unwrap_or_default();
        let mut new_gems: Vec<String> = Vec::new();
        for g in gems.iter().flat_map(|g| &g.gems).filter(|g| g.enabled) {
            if !before.contains(g.name.as_str()) && !new_gems.contains(&g.name) {
                new_gems.push(g.name.clone());
            }
        }

        let prev_gear: Vec<(String, u32)> = i
            .checked_sub(1)
            .and_then(|p| by_title(&b.item_sets, |s| &s.title, &b.specs[p].title, p))
            .map(|s| s.slots.clone())
            .unwrap_or_default();
        let gear = by_title(&b.item_sets, |s| &s.title, &spec.title, i)
            .map(|set| {
                set.slots
                    .iter()
                    .filter_map(|(slot, id)| {
                        let item = b.items.iter().find(|it| it.id == *id)?.clone();
                        let changed = !prev_gear.iter().any(|(s, pid)| s == slot && pid == id);
                        Some(GearTarget { slot: slot.clone(), item, changed })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let new_masteries = spec.masteries.iter().filter(|m| !prev.is_some_and(|p| p.masteries.contains(m)));
        let masteries = new_masteries
            .filter_map(|(node, effect)| {
                let name = tree.node(*node)?.name.clone();
                Some((name, tree.mastery_stats(*node, *effect)?.to_vec()))
            })
            .collect();

        stages.push(Stage {
            title: spec.title.clone(),
            from_level: level_from_title(&spec.title),
            passives: take_order(tree, prev, spec),
            masteries,
            gems,
            new_gems,
            gear,
        });
    }
    Plan {
        class: b.class.clone(),
        ascendancy: b.ascendancy.clone(),
        bandit: b.bandit.clone(),
        pantheon_major: b.pantheon_major.clone(),
        pantheon_minor: b.pantheon_minor.clone(),
        notes: b.notes.clone(),
        auto_order: b.specs.len() == 1,
        stages,
    }
}

impl Plan {
    /// Stage that applies at `level`: the last one whose title-derived level has been reached.
    /// Stages without a level in their title are never picked here, the player switches to them manually.
    pub fn stage_at(&self, level: u32) -> Option<usize> {
        let mut best = None;
        for (i, s) in self.stages.iter().enumerate() {
            if s.from_level.is_some_and(|l| l <= level) {
                best = Some(i);
            }
        }
        best.or_else(|| self.stages.iter().position(|s| s.from_level.is_some())).or((!self.stages.is_empty()).then_some(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pob_model::parse;

    fn plan(fixture: &str, tree: &str) -> Plan {
        let b = parse(&crate::pob_source::import(fixture).unwrap()).unwrap();
        build(&b, &TreeData::from_json(tree).unwrap())
    }

    #[test]
    fn levels_come_out_of_guide_style_titles() {
        let cases = [
            ("1) Level 1-12", Some(1)),
            ("lv 24+", Some(24)),
            ("lv 40+ (Merc Swap)", Some(40)),
            ("LVL.67+", Some(67)),
            ("2) Normal Lab", Some(33)),
            ("3) Cruel Lab", Some(55)),
            ("4) Merc Lab", Some(68)),
            ("1) Act 1-3 (up to firestorm swap)", Some(1)),
            ("Act 4", Some(33)),
            ("5) Pconc of Bouncing Swap", None),
            ("6) ENDGAME SETUP", None),
            ("Act 99", None),
            ("", None),
        ];
        for (title, want) in cases {
            assert_eq!(level_from_title(title), want, "{title}");
        }
    }

    #[test]
    fn real_ranger_plan_follows_levels_and_diffs_stages() {
        let p = plan(include_str!("../../fixtures/pob/real1.txt"), include_str!("../../fixtures/tree/nodes-real1.json"));
        assert_eq!(p.stages.len(), 7);
        assert_eq!(p.stages.iter().map(|s| s.from_level).collect::<Vec<_>>(), [Some(1), Some(33), Some(55), Some(68), None, None, None]);
        assert_eq!([5, 32, 33, 54, 55, 80, 99].map(|l| p.stage_at(l)), [Some(0), Some(0), Some(1), Some(1), Some(2), Some(3), Some(3)]);
        assert_eq!(p.stage_at(0), Some(0));
        // passives: first stage from scratch, later ones only the difference
        assert!(!p.stages[0].passives.is_empty());
        let first: HashSet<u32> = p.stages[0].passives.iter().map(|s| s.id).collect();
        assert!(p.stages[1].passives.iter().all(|s| !first.contains(&s.id)));
        // gems
        assert!(p.stages[0].new_gems.contains(&"Caustic Arrow".to_string()));
        assert!(!p.stages[1].new_gems.contains(&"Caustic Arrow".to_string()) || p.stages[0].gems.is_empty());
        // gear: the first stage's items are all "changed", Normal Lab keeps unchanged slots unchanged
        assert!(p.stages[0].gear.iter().all(|g| g.changed));
        assert!(p.stages[1].gear.iter().any(|g| !g.changed));
        assert!(p.stages[1].gear.iter().any(|g| g.slot == "Amulet"));
        // metadata
        assert_eq!((p.class.as_str(), p.bandit.as_str(), p.pantheon_major.as_str()), ("Ranger", "Alira", "TheBrineKing"));
        assert!(p.notes.starts_with("LEVELING NOTES"));
    }

    #[test]
    fn other_guide_styles_get_sensible_stage_levels() {
        let empty = "{\"nodes\":{}}";
        let scion = plan(include_str!("../../fixtures/pob/real3.txt"), empty);
        assert_eq!(scion.stages.iter().map(|s| s.from_level).collect::<Vec<_>>(), [Some(1), Some(24), Some(40), Some(67)]);
        assert_eq!([10, 24, 39, 40, 66, 67].map(|l| scion.stage_at(l)), [Some(0), Some(1), Some(1), Some(2), Some(2), Some(3)]);
        let witch = plan(include_str!("../../fixtures/pob/real2.txt"), empty);
        assert_eq!(witch.stages.iter().map(|s| s.from_level).collect::<Vec<_>>()[..4], [Some(1), Some(33), Some(55), Some(68)]);
    }

    #[test]
    fn single_tree_build_gets_a_connected_auto_order_covering_every_point() {
        let mut b = parse(&crate::pob_source::import(include_str!("../../fixtures/pob/real1.txt")).unwrap()).unwrap();
        let tree = TreeData::from_json(include_str!("../../fixtures/tree/nodes-real1.json")).unwrap();
        let endgame = b.specs.remove(5);
        let want: HashSet<u32> = endgame.nodes.iter().copied().filter(|id| tree.node(*id).is_some_and(|n| n.kind != crate::tree::Kind::Root)).collect();
        b.specs = vec![endgame];
        let p = build(&b, &tree);
        assert!(p.auto_order);
        assert_eq!(p.stages.len(), 1);
        assert_eq!(p.stage_at(1), Some(0)); // no level in the title, still the only stage
        let got: Vec<u32> = p.stages[0].passives.iter().filter(|s| s.known).map(|s| s.id).collect();
        assert_eq!(got.iter().copied().collect::<HashSet<_>>(), want);
        assert_eq!(got.len(), want.len(), "no duplicates");
        // every notable/keystone is reached before the plan is half over only if its road is: check the road-first heuristic
        let notables_in_first_half = p.stages[0].passives.iter().take(want.len() / 2).filter(|s| matches!(s.kind, crate::tree::Kind::Notable | crate::tree::Kind::Keystone)).count();
        let notables_total = p.stages[0].passives.iter().filter(|s| matches!(s.kind, crate::tree::Kind::Notable | crate::tree::Kind::Keystone)).count();
        assert!(notables_in_first_half * 2 > notables_total, "{notables_in_first_half}/{notables_total}");
        let multi = plan(include_str!("../../fixtures/pob/real1.txt"), include_str!("../../fixtures/tree/nodes-real1.json"));
        assert!(!multi.auto_order);
    }

    #[test]
    fn empty_build_has_no_stage() {
        let p = Plan { class: String::new(), ascendancy: String::new(), bandit: String::new(), pantheon_major: String::new(), pantheon_minor: String::new(), notes: String::new(), auto_order: false, stages: vec![] };
        assert_eq!(p.stage_at(50), None);
    }
}
