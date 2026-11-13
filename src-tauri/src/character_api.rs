//! `GET /character/<name>` of GGG's OAuth API (scope account:characters) turned into what the plan needs.
//! Parsing only: no network, no login. The fields follow https://www.pathofexile.com/developer/docs/reference
//! (checked 2026-11-13); the test fixture is hand-written from that description, not a real response.

#![allow(dead_code)] // shortcut: unused until the sync fetch (T35) and the diff (T36) call it, remove then

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::pob_model::{Gem, GemGroup};

#[derive(Deserialize)]
struct Envelope {
    character: Raw,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Raw {
    name: String,
    class: String,
    level: u32,
    equipment: Vec<RawItem>,
    passives: RawPassives,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawPassives {
    hashes: Vec<u32>,
    hashes_ex: Vec<u32>,
    mastery_effects: std::collections::HashMap<String, u32>,
    bandit_choice: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawItem {
    inventory_id: String,
    sockets: Vec<RawSocket>,
    #[serde(rename = "socketedItems")]
    socketed_items: Vec<RawGem>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawSocket {
    group: u32,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawGem {
    #[serde(rename = "typeLine")]
    type_line: String,
    support: bool,
    socket: usize,
    properties: Vec<RawProp>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawProp {
    name: String,
    values: Vec<(String, u32)>,
}

/// What the character really has, in the shapes the plan already uses.
#[derive(Debug, Default, Serialize)]
pub struct Synced {
    pub name: String,
    pub class: String,
    pub level: u32,
    /// every allocated passive id (regular and ascendancy)
    pub nodes: HashSet<u32>,
    /// (mastery node, chosen effect)
    pub masteries: Vec<(u32, u32)>,
    pub bandit: Option<String>,
    pub groups: Vec<GemGroup>,
}

pub fn parse(json: &str) -> Result<Synced, serde_json::Error> {
    let Raw { name, class, level, equipment, passives } = serde_json::from_str::<Envelope>(json)?.character;
    let mut groups = Vec::new();
    for item in equipment {
        // gems link when their sockets share a group number; one GemGroup per linked set, labelled by the slot
        let mut by_group: Vec<(u32, Vec<Gem>)> = Vec::new();
        for g in item.socketed_items {
            let group = item.sockets.get(g.socket).map_or(0, |s| s.group);
            let prop = |n: &str| g.properties.iter().find(|p| p.name == n).and_then(|p| p.values.first()).and_then(|v| v.0.trim_end_matches('%').trim_start_matches('+').parse().ok()).unwrap_or(0);
            let gem = Gem { level: prop("Level"), quality: prop("Quality"), name: g.type_line, support: g.support, enabled: true };
            match by_group.iter_mut().find(|(k, _)| *k == group) {
                Some((_, v)) => v.push(gem),
                None => by_group.push((group, vec![gem])),
            }
        }
        groups.extend(by_group.into_iter().map(|(_, gems)| GemGroup { slot: item.inventory_id.clone(), enabled: true, gems }));
    }
    let masteries = passives.mastery_effects.iter().filter_map(|(k, v)| Some((k.parse().ok()?, *v))).collect();
    let nodes = passives.hashes.into_iter().chain(passives.hashes_ex).collect();
    Ok(Synced { name, class, level, nodes, masteries, bandit: passives.bandit_choice, groups })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"character":{"name":"Zed","class":"Pathfinder","level":40,"league":"Standard",
      "equipment":[{"inventory_id":"BodyArmour","sockets":[{"group":0},{"group":0},{"group":1}],
        "socketedItems":[
          {"typeLine":"Caustic Arrow","support":false,"socket":0,"properties":[{"name":"Level","values":[["12",0]]},{"name":"Quality","values":[["+5%",1]]}]},
          {"typeLine":"Added Chaos Damage Support","support":true,"socket":1,"properties":[{"name":"Level","values":[["8",0]]}]},
          {"typeLine":"Herald of Agony","support":false,"socket":2}]},
        {"inventory_id":"Helm"}],
      "passives":{"hashes":[100,200],"hashes_ex":[300],"mastery_effects":{"4":55,"bad":1},"bandit_choice":"Alira"}}}"#;

    #[test]
    fn maps_nodes_masteries_bandit_and_linked_gems() {
        let s = parse(SAMPLE).unwrap();
        assert_eq!((s.name.as_str(), s.class.as_str(), s.level), ("Zed", "Pathfinder", 40));
        assert_eq!(s.nodes, HashSet::from([100, 200, 300]));
        assert_eq!(s.masteries, vec![(4, 55)]);
        assert_eq!(s.bandit.as_deref(), Some("Alira"));
        assert_eq!(s.groups.len(), 2);
        let first = &s.groups[0];
        assert_eq!(first.slot, "BodyArmour");
        assert_eq!(first.gems.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Caustic Arrow", "Added Chaos Damage Support"]);
        assert_eq!((first.gems[0].level, first.gems[0].quality, first.gems[1].support), (12, 5, true));
        assert_eq!(s.groups[1].gems[0].name, "Herald of Agony");
    }

    #[test]
    fn missing_fields_and_garbage() {
        assert_eq!(parse(r#"{"character":{}}"#).unwrap().groups.len(), 0);
        assert!(parse("not json").is_err());
    }
}
