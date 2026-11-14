//! `GET /character/<name>` of GGG's OAuth API (scope account:characters) turned into what the plan needs.
//! Parsing only: no network, no login. The fields follow https://www.pathofexile.com/developer/docs/reference
//! (checked 2026-11-13); the test fixture is hand-written from that description, not a real response.


use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::plan::Plan;
use crate::pob_model::{Gem, GemGroup};
use crate::tree::{Kind, TreeData};

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
    // an object; the site sends `[]` when empty and may send the effect id as a string
    mastery_effects: serde_json::Value,
    bandit_choice: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawItem {
    #[serde(alias = "inventoryId")]
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
    Ok(assemble(serde_json::from_str::<Envelope>(json)?.character))
}

fn assemble(raw: Raw) -> Synced {
    let Raw { name, class, level, equipment, passives } = raw;
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
    let masteries = passives.mastery_effects.as_object().into_iter().flatten().filter_map(|(k, v)| Some((k.parse().ok()?, v.as_u64().or_else(|| v.as_str()?.parse().ok())? as u32))).collect();
    let nodes = passives.hashes.into_iter().chain(passives.hashes_ex).collect();
    Synced { name, class, level, nodes, masteries, bandit: passives.bandit_choice, groups }
}

/// The two website responses (`get-items`, `get-passive-skills`) of a public profile, the same result as `parse`.
pub fn parse_public(items: &str, passives: &str) -> Result<Synced, serde_json::Error> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Items {
        items: Vec<RawItem>,
        character: Raw,
    }
    let Items { items, character } = serde_json::from_str(items)?;
    Ok(assemble(Raw { equipment: items, passives: serde_json::from_str(passives)?, ..character }))
}

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// Shown when the site answers 403; the caller may then try again once with a stored cookie.
pub const PRIVATE: &str = "Profile not readable: make the characters tab public in your account settings (or save your POESESSID below), and check both names";

/// Blocking. Reads a profile of the PoE1 realm: two GETs, no retry. Public profiles need no login; `cookie` (POESESSID) is for private ones.
/// Any 4xx stops here; the site rate-limits per IP (15 requests per 60 s at the time of writing).
pub fn fetch_public(account: &str, character: &str, cookie: Option<&str>) -> Result<Synced, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("PoeMate/", env!("CARGO_PKG_VERSION"), " (public profile sync)"))
        .build()
        .map_err(|e| e.to_string())?;
    let get = |page: &str| -> Result<String, String> {
        let url = format!("https://www.pathofexile.com/character-window/{page}?accountName={}&character={}&realm=pc", enc(account), enc(character));
        let mut req = client.get(url);
        if let Some(c) = cookie {
            req = req.header("Cookie", format!("POESESSID={c}"));
        }
        let resp = req.send().map_err(|_| "Couldn't reach pathofexile.com (offline or timed out)".to_string())?;
        match resp.status().as_u16() {
            200 => {}
            403 => return Err(PRIVATE.into()),
            404 => return Err("Account or character not found".into()),
            429 => return Err("Too many requests: wait a minute and try again".into()),
            n => return Err(format!("pathofexile.com answered {n}")),
        }
        let mut body = Vec::new();
        std::io::Read::read_to_end(&mut std::io::Read::take(resp, 4_000_000), &mut body).map_err(|e| e.to_string())?;
        String::from_utf8(body).map_err(|e| e.to_string())
    };
    let items = get("get-items")?;
    let passives = get("get-passive-skills")?;
    parse_public(&items, &passives).map_err(|e| format!("Unexpected answer from pathofexile.com: {e}"))
}

impl Synced {
    /// Class and ascendancy starts come with the character for free: they are never a choice, so never "extra".
    pub fn drop_free(&mut self, tree: &TreeData) {
        self.nodes.retain(|n| tree.node(*n).is_none_or(|x| x.kind != Kind::Root));
    }
}

/// Plan vs character at one stage. Passives are compared cumulatively (stages only list the points they add).
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Diff {
    /// planned up to this stage, not allocated yet
    pub missing: Vec<u32>,
    /// allocated but not in the plan
    pub extra: Vec<u32>,
    pub missing_gems: Vec<String>,
}

/// PoB names a support "Pierce", the game "Pierce Support"; alternate quality gems carry a prefix.
fn gem_key(name: &str) -> &str {
    let name = ["Anomalous ", "Divergent ", "Phantasmal "].iter().find_map(|p| name.strip_prefix(p)).unwrap_or(name);
    name.strip_suffix(" Support").unwrap_or(name)
}

pub fn diff(plan: &Plan, stage: usize, have: &Synced) -> Diff {
    let planned_nodes: Vec<u32> = plan.stages.iter().take(stage + 1).flat_map(|s| s.passives.iter().map(|p| p.id)).collect();
    let planned: HashSet<u32> = planned_nodes.iter().copied().collect();
    let mut extra: Vec<u32> = have.nodes.iter().copied().filter(|n| !planned.contains(n)).collect();
    extra.sort_unstable();
    let owned: HashSet<&str> = have.groups.iter().flat_map(|g| g.gems.iter().map(|g| gem_key(&g.name))).collect();
    let mut missing_gems: Vec<String> = Vec::new();
    for g in plan.stages.get(stage).iter().flat_map(|s| &s.gems).filter(|g| g.enabled).flat_map(|g| &g.gems).filter(|g| g.enabled) {
        if !owned.contains(gem_key(&g.name)) && !missing_gems.contains(&g.name) {
            missing_gems.push(g.name.clone());
        }
    }
    Diff { missing: planned_nodes.into_iter().filter(|n| !have.nodes.contains(n)).collect(), extra, missing_gems }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Stage;
    use crate::tree::Step;

    fn step(id: u32) -> Step {
        Step { id, name: String::new(), kind: Kind::Normal, stats: vec![], ascendancy: None, known: false, pos: None, from: None }
    }

    fn stage(ids: &[u32], gems: &[&str]) -> Stage {
        let gem = |n: &&str| Gem { name: n.to_string(), enabled: true, ..Default::default() };
        Stage {
            title: String::new(),
            from_level: None,
            passives: ids.iter().map(|i| step(*i)).collect(),
            masteries: vec![],
            gems: vec![GemGroup { slot: String::new(), enabled: true, gems: gems.iter().map(gem).collect() }],
            new_gems: vec![],
            gear: vec![],
        }
    }

    #[test]
    fn diff_is_cumulative_and_lists_missing_gems() {
        let plan = Plan {
            class: String::new(),
            ascendancy: String::new(),
            bandit: String::new(),
            pantheon_major: String::new(),
            pantheon_minor: String::new(),
            notes: String::new(),
            auto_order: false,
            stages: vec![stage(&[100, 200], &["Caustic Arrow"]), stage(&[300, 400], &["Caustic Arrow", "Herald of Agony", "Pride"])],
        };
        let have = parse(SAMPLE).unwrap(); // nodes 100, 200, 300; gems Caustic Arrow, Added Chaos, Herald of Agony
        let d0 = diff(&plan, 0, &have);
        assert_eq!((d0.missing, d0.extra, d0.missing_gems), (vec![], vec![300], vec![]));
        let d1 = diff(&plan, 1, &have);
        assert_eq!((d1.missing, d1.extra, d1.missing_gems), (vec![400], vec![], vec!["Pride".to_string()]));
    }

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
    fn gems_match_across_support_suffix_and_quality_prefix() {
        assert_eq!((gem_key("Pierce Support"), gem_key("Pierce"), gem_key("Anomalous Caustic Arrow")), ("Pierce", "Pierce", "Caustic Arrow"));
        let mut plan = Plan { class: String::new(), ascendancy: String::new(), bandit: String::new(), pantheon_major: String::new(), pantheon_minor: String::new(), notes: String::new(), auto_order: false, stages: vec![stage(&[], &["Pierce", "Caustic Arrow", "Pride"])] };
        plan.stages[0].gems[0].gems.truncate(3);
        let have = Synced { groups: vec![GemGroup { slot: String::new(), enabled: true, gems: ["Pierce Support", "Anomalous Caustic Arrow"].map(|n| Gem { name: n.into(), ..Default::default() }).into() }], ..Default::default() };
        assert_eq!(diff(&plan, 0, &have).missing_gems, ["Pride"]);
    }

    #[test]
    fn free_start_nodes_are_dropped() {
        let tree = TreeData::from_json(r#"{"nodes":{"1":{"skill":1,"classStartIndex":0},"100":{"skill":100}}}"#).unwrap();
        let mut s = Synced { nodes: HashSet::from([1, 100, 999]), ..Default::default() };
        s.drop_free(&tree);
        assert_eq!(s.nodes, HashSet::from([100, 999]));
    }

    #[test]
    fn public_profile_responses_are_read_like_the_api_ones() {
        let items = r#"{"items":[{"inventoryId":"Helm","sockets":[{"group":0},{"group":0}],"socketedItems":[{"typeLine":"Caustic Arrow","support":false,"socket":0,"properties":[{"name":"Level","values":[["12",0]]}]},{"typeLine":"Pierce Support","support":true,"socket":1}]}],"character":{"name":"Momkay","class":"Deadeye","level":40}}"#;
        let passives = r#"{"hashes":[10,20],"hashes_ex":[30],"mastery_effects":{"5":"77"}}"#;
        assert!(parse_public(items, r#"{"hashes":[],"mastery_effects":[]}"#).unwrap().masteries.is_empty());
        let s = parse_public(items, passives).unwrap();
        assert_eq!((s.name.as_str(), s.class.as_str(), s.level), ("Momkay", "Deadeye", 40));
        assert_eq!(s.nodes, HashSet::from([10, 20, 30]));
        assert_eq!(s.masteries, [(5, 77)]);
        assert_eq!((s.groups[0].slot.as_str(), s.groups[0].gems.len(), s.groups[0].gems[0].level), ("Helm", 2, 12));
        assert_eq!(enc("Momkay#1234"), "Momkay%231234");
    }

    #[test]
    fn missing_fields_and_garbage() {
        assert_eq!(parse(r#"{"character":{}}"#).unwrap().groups.len(), 0);
        assert!(parse("not json").is_err());
    }
}
