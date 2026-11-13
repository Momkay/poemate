//! Passive point budget: 1 point per level (from level 2) plus the quest rewards.
//!
//! Quest table: https://www.poewiki.net/wiki/Passive_skill (read 2026-11-13): 23 points from quests, +1 for
//! "Deal with the Bandits" only when all three bandits are killed. A quest counts once its step in the route is behind
//! the cursor (approximation: the point is given when the quest is handed in or completed there).

use std::collections::HashSet;

use crate::route::{Kind, Part, Step};

/// (quest id in the route data, passive points)
pub const QUEST_POINTS: &[(&str, u32)] = &[
    ("a1q7", 1),  // The Dweller of the Deep
    ("a1q6", 1),  // The Marooned Mariner
    ("a1q9", 1),  // The Way Forward
    ("a2q5", 1),  // Through Sacred Ground
    ("a2q7", 1),  // Deal with the Bandits (kill all three only)
    ("a3q11", 1), // Victario's Secrets
    ("a3q9", 1),  // Piety's Pets
    ("a4q6", 1),  // An Indomitable Spirit
    ("a5q3", 1),  // In Service to Science
    ("a5q7", 1),  // Kitava's Torments
    ("a6q3", 1),  // The Father of War
    ("a6q7", 1),  // The Cloven One
    ("a6q6", 1),  // The Puppet Mistress
    ("a7q1", 1),  // The Master of a Million Faces
    ("a7q9", 1),  // Queen of Despair
    ("a7q6", 1),  // Kishara's Star
    ("a8q6", 1),  // Love is Dead
    ("a8q4", 1),  // Reflection of Terror
    ("a8q7", 1),  // The Gemling Legion
    ("a9q5", 1),  // Queen of the Sands
    ("a9q2", 1),  // The Ruler of Highgate
    ("a10q6", 1), // Vilenta's Vengeance
    ("a10q3", 2), // An End to Hunger
];
const BANDIT_QUEST: &str = "a2q7";

pub fn quest_points(id: &str, flags: &HashSet<String>) -> u32 {
    if id == BANDIT_QUEST && !flags.contains("BANDIT_KILL") {
        return 0;
    }
    QUEST_POINTS.iter().find(|(q, _)| *q == id).map(|(_, p)| *p).unwrap_or(0)
}

fn quest_ids(step: &Step) -> impl Iterator<Item = &str> {
    step.parts.iter().filter_map(|p| match p {
        Part::Frag { kind: Kind::Quest, args } => Some(args),
        _ => None,
    }).flatten().map(String::as_str)
}

/// Points from the quests whose steps are in `steps` (each quest once).
pub fn earned(steps: &[&Step], flags: &HashSet<String>) -> u32 {
    let mut seen: HashSet<&str> = HashSet::new();
    steps.iter().flat_map(|s| quest_ids(s)).filter(|id| seen.insert(id)).map(|id| quest_points(id, flags)).sum()
}

/// Points the quests of this one step give (shown as "+1 passive point" when the step is next).
pub fn at_step(step: &Step, flags: &HashSet<String>) -> u32 {
    quest_ids(step).map(|id| quest_points(id, flags)).sum()
}

pub fn level_points(level: u32) -> u32 {
    level.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::Route;
    use crate::route_engine::{flags_for, Engine, Progress};

    #[test]
    fn totals_match_the_wiki() {
        let kill = flags_for("None", false);
        let alira = flags_for("Alira", false);
        let sum = |f: &HashSet<String>| QUEST_POINTS.iter().map(|(id, _)| quest_points(id, f)).sum::<u32>();
        assert_eq!(sum(&kill), 24);
        assert_eq!(sum(&alira), 23);
    }

    #[test]
    fn every_listed_quest_is_a_step_in_the_route_and_the_route_pays_out_everything() {
        let route = Route::load().unwrap();
        for (flags, total) in [(flags_for("None", false), 24), (flags_for("Oak", true), 23)] {
            let e = Engine::new(&route, &flags, Progress { cursor: usize::MAX });
            let steps: Vec<&Step> = route.steps.iter().filter(|s| s.applies(&flags)).collect();
            assert_eq!(earned(&steps, &flags), total, "{flags:?}");
            let _ = e;
        }
    }

    #[test]
    fn points_arrive_when_the_hand_in_step_is_passed() {
        let route = Route::load().unwrap();
        let flags = flags_for("Alira", false);
        let steps: Vec<&Step> = route.steps.iter().filter(|s| s.applies(&flags)).collect();
        let dweller = steps.iter().position(|s| quest_ids(s).any(|q| q == "a1q7")).unwrap();
        assert_eq!(earned(&steps[..dweller], &flags), 0); // Marooned Mariner comes later in the route
        assert_eq!(at_step(steps[dweller], &flags), 1);
        assert_eq!(earned(&steps[..=dweller], &flags), 1);
    }

    #[test]
    fn a_quest_listed_twice_pays_once_and_levels_give_one_point_each() {
        let route = Route::load().unwrap();
        let flags = flags_for("Alira", false);
        let steps: Vec<&Step> = route.steps.iter().filter(|s| quest_ids(s).any(|q| q == "a1q9")).collect(); // all variants of the route
        assert!(steps.len() > 1, "The Way Forward appears in several steps");
        assert_eq!(earned(&steps, &flags), 1);
        assert_eq!((level_points(0), level_points(1), level_points(2), level_points(100)), (0, 0, 1, 99));
    }
}
