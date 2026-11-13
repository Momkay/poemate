//! Quest reward and vendor gem offers (Exile Leveling data) filtered to the gems the build wants.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Quest {
    reward_offers: HashMap<String, Offers>,
}

#[derive(Deserialize)]
struct Offers {
    #[serde(default)]
    quest_npc: String,
    #[serde(default)]
    quest: HashMap<String, GemInfo>,
    #[serde(default)]
    vendor: HashMap<String, GemInfo>,
}

#[derive(Deserialize)]
struct GemInfo {
    #[serde(default)]
    classes: Vec<String>,
    #[serde(default)]
    npc: String,
}

#[derive(Deserialize)]
struct Gem {
    name: String,
}

static QUESTS: LazyLock<HashMap<String, Quest>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../../data/quests.json")).expect("bundled quests.json is valid"));
static GEMS: LazyLock<HashMap<String, Gem>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../../data/gems.json")).expect("bundled gems.json is valid"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum How {
    /// pick it as the quest reward
    Take,
    /// buy it from the vendor after the quest
    Buy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Offer {
    pub gem: String,
    pub how: How,
    pub npc: String,
}

/// "Frost Bolt" and "Frostbolt" are the same gem to us.
pub fn norm(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase()
}

/// Offers for the given quest ids that `class` can use and the build wants (`wanted` holds `norm`ed gem names).
/// Rewards come before vendor purchases; a gem is listed once.
pub fn offers(quest_ids: &[String], class: &str, wanted: &HashSet<String>) -> Vec<Offer> {
    let mut out: Vec<Offer> = Vec::new();
    let usable = |gem_id: &str, info: &GemInfo| -> Option<String> {
        let g = GEMS.get(gem_id)?;
        (info.classes.iter().any(|c| c == class) && wanted.contains(&norm(&g.name))).then(|| g.name.clone())
    };
    for how in [How::Take, How::Buy] {
        for id in quest_ids {
            let Some(q) = QUESTS.get(id) else { continue };
            let mut offers: Vec<_> = q.reward_offers.values().collect();
            offers.sort_by(|a, b| a.quest_npc.cmp(&b.quest_npc));
            for o in offers {
                let table = if how == How::Take { &o.quest } else { &o.vendor };
                let mut gems: Vec<_> = table.iter().collect();
                gems.sort_by(|a, b| a.0.cmp(b.0));
                for (gem_id, info) in gems {
                    if let Some(name) = usable(gem_id, info) {
                        if !out.iter().any(|x| x.gem == name) {
                            let npc = if how == How::Buy && !info.npc.is_empty() { info.npc.clone() } else { o.quest_npc.clone() };
                            out.push(Offer { gem: name, how, npc });
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn want(names: &[&str]) -> HashSet<String> {
        names.iter().map(|n| norm(n)).collect()
    }

    #[test]
    fn gem_names_match_regardless_of_spacing_and_case() {
        assert_eq!(norm("Frost Bolt"), norm("frostbolt"));
        assert_eq!(norm("Sniper's Mark"), "snipersmark");
    }

    #[test]
    fn data_loads_and_knows_the_first_quests() {
        assert!(QUESTS.len() > 50 && GEMS.len() > 500);
        assert!(QUESTS.contains_key("a1q1") && QUESTS.contains_key("a1q5"));
    }

    #[test]
    fn only_class_usable_wanted_gems_are_offered_and_rewards_precede_purchases() {
        // Enemy at the Gate: Tarkleigh lets a Ranger take Caustic Arrow and Sniper's Mark
        let ids = ["a1q1".to_string()];
        let w = want(&["Caustic Arrow", "Sniper's Mark", "Fireball"]);
        let ranger = offers(&ids, "Ranger", &w);
        assert!(ranger.iter().any(|o| o.gem == "Caustic Arrow" && o.how == How::Take && o.npc == "Tarkleigh"), "{ranger:?}");
        assert!(!ranger.iter().any(|o| o.gem == "Fireball"), "wanted but not offered to a Ranger");
        let witch = offers(&ids, "Witch", &w);
        assert!(!witch.iter().any(|o| o.gem == "Caustic Arrow"));
        assert!(offers(&ids, "Ranger", &want(&[])).is_empty());
        assert!(offers(&["nonexistent".to_string()], "Ranger", &w).is_empty());
        // purchases come after rewards
        let all: Vec<String> = QUESTS.keys().cloned().collect();
        let both = offers(&all, "Ranger", &want(&["Caustic Arrow", "Burning Arrow"]));
        let pos = |g: &str| both.iter().position(|o| o.gem == g);
        assert_eq!(both.iter().find(|o| o.gem == "Burning Arrow").map(|o| o.how), Some(How::Buy));
        assert!(pos("Caustic Arrow") < pos("Burning Arrow"));
    }
    #[test]
    fn the_rangers_real_build_finds_offers_across_the_campaign() {
        let b = crate::pob_model::parse(&crate::pob_source::import(include_str!("../../fixtures/pob/real1.txt")).unwrap()).unwrap();
        let wanted: HashSet<String> = b.skill_sets[0].groups.iter().flat_map(|g| g.gems.iter().map(|x| norm(&x.name))).collect();
        let all_quests: Vec<String> = QUESTS.keys().cloned().collect();
        let found = offers(&all_quests, &b.class, &wanted);
        assert!(!found.is_empty());
        assert!(found.iter().all(|o| wanted.contains(&norm(&o.gem))));
        // listed once
        let names: HashSet<_> = found.iter().map(|o| &o.gem).collect();
        assert_eq!(names.len(), found.len());
        eprintln!("{found:?}");
    }
}
