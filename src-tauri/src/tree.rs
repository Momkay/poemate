//! Passive tree data (GGG's skilltree-export) and the "which points do I take next" logic.
//!
//! Source: https://github.com/grindinggear/skilltree-export (public, latest tree only; PoB spec ids resolve
//! against it. Nodes it doesn't know, e.g. cluster jewel nodes above id 65535, are reported as unknown).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::pob_model::Spec;

const URL: &str = "https://raw.githubusercontent.com/grindinggear/skilltree-export/master/data.json";
const CACHE_FILE: &str = "skilltree.json";
const MAX_DOWNLOAD: u64 = 32 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Kind {
    Normal,
    Notable,
    Keystone,
    Mastery,
    Jewel,
    /// class start or ascendancy start: free, never a choice
    Root,
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: u32,
    pub name: String,
    pub kind: Kind,
    pub stats: Vec<String>,
    pub ascendancy: Option<String>,
    #[serde(skip)]
    links: Vec<u32>,
    #[serde(skip)]
    effects: Vec<(u32, Vec<String>)>,
}

#[derive(Deserialize)]
struct RawFile {
    nodes: HashMap<String, RawNode>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawNode {
    skill: u32,
    name: String,
    stats: Vec<String>,
    is_notable: bool,
    is_keystone: bool,
    is_mastery: bool,
    is_jewel_socket: bool,
    ascendancy_name: Option<String>,
    is_ascendancy_start: bool,
    class_start_index: Option<u32>,
    out: Vec<String>,
    #[serde(rename = "in")]
    inn: Vec<String>,
    mastery_effects: Vec<RawEffect>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawEffect {
    effect: u32,
    stats: Vec<String>,
}

pub struct TreeData {
    nodes: HashMap<u32, Node>,
}

#[derive(Debug)]
pub enum TreeError {
    Parse,
    Offline,
}

impl TreeData {
    pub fn from_json(json: &str) -> Result<Self, TreeError> {
        let raw: RawFile = serde_json::from_str(json).map_err(|_| TreeError::Parse)?;
        let nodes = raw
            .nodes
            .into_values()
            .filter(|n| n.skill != 0)
            .map(|n| {
                let kind = if n.class_start_index.is_some() || n.is_ascendancy_start {
                    Kind::Root
                } else if n.is_keystone {
                    Kind::Keystone
                } else if n.is_mastery {
                    Kind::Mastery
                } else if n.is_jewel_socket {
                    Kind::Jewel
                } else if n.is_notable {
                    Kind::Notable
                } else {
                    Kind::Normal
                };
                let links = n.out.iter().chain(&n.inn).filter_map(|s| s.parse().ok()).collect();
                let effects = n.mastery_effects.into_iter().map(|e| (e.effect, e.stats)).collect();
                (n.skill, Node { id: n.skill, name: n.name, kind, stats: n.stats, ascendancy: n.ascendancy_name, links, effects })
            })
            .collect();
        Ok(Self { nodes })
    }

    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Stats of the effect chosen on a mastery node.
    pub fn mastery_stats(&self, node: u32, effect: u32) -> Option<&[String]> {
        self.nodes.get(&node)?.effects.iter().find(|(e, _)| *e == effect).map(|(_, s)| s.as_slice())
    }
}

/// Cached copy first; downloads when missing or `force`. A failed refresh falls back to the cache.
pub fn load(cache_dir: &Path, force: bool, fetch: impl Fn() -> Result<String, TreeError>) -> Result<TreeData, TreeError> {
    let path = cache_dir.join(CACHE_FILE);
    let cached = std::fs::read_to_string(&path).ok();
    if let (Some(json), false) = (&cached, force) {
        if let Ok(t) = TreeData::from_json(json) {
            return Ok(t);
        }
    }
    match fetch().and_then(|json| TreeData::from_json(&json).map(|t| (json, t))) {
        Ok((json, t)) => {
            let _ = std::fs::create_dir_all(cache_dir);
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
            Ok(t)
        }
        Err(e) => cached.as_deref().map(TreeData::from_json).transpose()?.ok_or(e),
    }
}

pub fn download() -> Result<String, TreeError> {
    use std::io::Read;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("PoeMate/", env!("CARGO_PKG_VERSION"), " (passive tree data)"))
        .build()
        .map_err(|_| TreeError::Offline)?;
    let resp = client.get(URL).send().map_err(|_| TreeError::Offline)?;
    if resp.status().as_u16() != 200 {
        return Err(TreeError::Offline);
    }
    let mut body = Vec::new();
    resp.take(MAX_DOWNLOAD + 1).read_to_end(&mut body).map_err(|_| TreeError::Offline)?;
    if body.len() as u64 > MAX_DOWNLOAD {
        return Err(TreeError::Parse);
    }
    String::from_utf8(body).map_err(|_| TreeError::Parse)
}

/// One passive to allocate. `node` is None when the tree data doesn't know the id.
#[derive(Debug, Serialize)]
pub struct Step {
    pub id: u32,
    pub name: String,
    pub kind: Kind,
    pub stats: Vec<String>,
    pub ascendancy: Option<String>,
    pub known: bool,
}

fn step(tree: &TreeData, id: u32) -> Step {
    match tree.node(id) {
        Some(n) => Step { id, name: n.name.clone(), kind: n.kind, stats: n.stats.clone(), ascendancy: n.ascendancy.clone(), known: true },
        None => Step {
            id,
            name: if id > 65535 { "Cluster jewel passive".into() } else { format!("Unknown passive #{id}") },
            kind: Kind::Normal,
            stats: Vec::new(),
            ascendancy: None,
            known: false,
        },
    }
}

/// Steps from `start` to the nearest notable or keystone, walking only through points still to be taken.
/// Taking the road to a notable first means every point spent soon pays off.
fn steps_to_notable(tree: &TreeData, start: u32, left: &HashSet<u32>) -> u32 {
    let mut seen = HashSet::from([start]);
    let mut frontier = vec![start];
    let mut dist = 0;
    while !frontier.is_empty() {
        if frontier.iter().any(|id| tree.node(*id).is_some_and(|n| matches!(n.kind, Kind::Notable | Kind::Keystone))) {
            return dist;
        }
        let mut next = Vec::new();
        for id in frontier {
            for l in tree.node(id).map(|n| n.links.as_slice()).unwrap_or(&[]) {
                if left.contains(l) && seen.insert(*l) {
                    next.push(*l);
                }
            }
        }
        frontier = next;
        dist += 1;
    }
    u32::MAX
}

/// Points to take to go from `prev` (None = a fresh character) to `next`, in allocation order:
/// each step touches the already allocated tree. PoB doesn't store the order, so this is a
/// deterministic reconstruction (the road to the nearest notable first, lowest id on ties). Nodes that can't be reached
/// (cluster jewels, unknown ids) come last.
pub fn take_order(tree: &TreeData, prev: Option<&Spec>, next: &Spec) -> Vec<Step> {
    let before: HashSet<u32> = prev.map(|p| p.nodes.iter().copied().collect()).unwrap_or_default();
    let mut have: HashSet<u32> = before.clone();
    let mut todo: Vec<u32> = Vec::new();
    for &id in &next.nodes {
        if before.contains(&id) {
            continue;
        }
        if tree.node(id).is_some_and(|n| n.kind == Kind::Root) {
            have.insert(id); // free
        } else {
            todo.push(id);
        }
    }
    todo.sort_unstable();
    todo.dedup();

    let mut out = Vec::with_capacity(todo.len());
    while !todo.is_empty() {
        let left: HashSet<u32> = todo.iter().copied().collect();
        let next_idx = (0..todo.len())
            .filter(|&i| tree.node(todo[i]).is_some_and(|n| n.links.iter().any(|l| have.contains(l))))
            .min_by_key(|&i| (steps_to_notable(tree, todo[i], &left), todo[i]));
        let Some(i) = next_idx else { break };
        let id = todo.remove(i);
        have.insert(id);
        out.push(step(tree, id));
    }
    out.extend(todo.into_iter().map(|id| step(tree, id)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pob_model::parse;

    const FIXTURE: &str = include_str!("../../fixtures/tree/nodes-real1.json");

    fn tree() -> TreeData {
        TreeData::from_json(FIXTURE).unwrap()
    }

    fn build() -> crate::pob_model::Build {
        parse(&crate::pob_source::import(include_str!("../../fixtures/pob/real1.txt")).unwrap()).unwrap()
    }

    #[test]
    fn resolves_names_kinds_and_mastery_effects() {
        let t = tree();
        let n = t.node(41536).unwrap();
        assert_eq!((n.name.as_str(), n.kind), ("Projectile Damage", Kind::Normal));
        assert_eq!(n.stats, ["10% increased Projectile Damage"]);
        assert_eq!(t.node(41016).unwrap().kind, Kind::Mastery);
        assert_eq!(t.node(50459).unwrap().kind, Kind::Root);
        assert_eq!(t.node(64111).unwrap().kind, Kind::Root);
        assert_eq!(t.mastery_stats(41016, 29214).unwrap()[0], "Life Flasks gain 1 Charge every 3 seconds");
        assert!(t.node(1).is_none());
    }

    #[test]
    fn every_stage_of_the_real_build_is_a_connected_allocation_order() {
        let (t, b) = (tree(), build());
        let mut prev: Option<&Spec> = None;
        let mut total = 0;
        for spec in &b.specs {
            let steps = take_order(&t, prev, spec);
            // only cluster jewel nodes (ids above 65535) may be unknown, and they come last
            assert!(steps.iter().all(|s| s.known || s.id > 65535), "{}: unknown node", spec.title);
            assert!(steps.iter().all(|s| s.kind != Kind::Root));
            let first_unknown = steps.iter().position(|s| !s.known).unwrap_or(steps.len());
            assert!(steps[first_unknown..].iter().all(|s| !s.known), "{}: unknown nodes must be last", spec.title);
            let steps: Vec<_> = steps.into_iter().filter(|s| s.known).collect();
            // replaying the order: each step must touch roots or earlier steps
            let mut have: HashSet<u32> = prev.map(|p| p.nodes.iter().copied().collect()).unwrap_or_default();
            have.extend(spec.nodes.iter().filter(|id| t.node(**id).is_some_and(|n| n.kind == Kind::Root)));
            for s in &steps {
                // a jewel socket may be reached through a cluster jewel, which the tree data can't show
                let reachable = t.node(s.id).unwrap().links.iter().any(|l| have.contains(l));
                assert!(reachable || s.kind == Kind::Jewel, "{}: {} floats", spec.title, s.name);
                have.insert(s.id);
            }
            total += steps.len();
            prev = Some(spec);
        }
        assert!(total > 100);
    }

    #[test]
    fn first_stage_takes_only_new_points_and_later_stages_only_the_difference() {
        let (t, b) = (tree(), build());
        let first = take_order(&t, None, &b.specs[0]);
        // 14 nodes in the spec, minus class start, ascendancy start and Breachlord-style free roots
        assert!(first.len() < b.specs[0].nodes.len() && first.len() >= 9, "{}", first.len());
        let second = take_order(&t, Some(&b.specs[0]), &b.specs[1]);
        let already: HashSet<u32> = b.specs[0].nodes.iter().copied().collect();
        assert!(second.iter().all(|s| !already.contains(&s.id)));
        assert!(!second.is_empty());
    }

    #[test]
    fn unknown_and_cluster_ids_are_reported_last_not_dropped() {
        let t = tree();
        let spec = Spec { nodes: vec![60_001, 70_000, 41536], ..Default::default() };
        let steps = take_order(&t, None, &spec);
        // 41536 floats too (no allocated neighbour in this tiny spec) but is a known node; all three are kept
        assert_eq!(steps.len(), 3);
        assert!(steps.iter().any(|s| s.id == 70_000 && s.name == "Cluster jewel passive" && !s.known));
        assert!(steps.iter().any(|s| s.id == 60_001 && s.name == "Unknown passive #60001" && !s.known));
    }

    #[test]
    fn cache_is_used_before_network_and_survives_a_failed_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let calls = std::cell::Cell::new(0);
        let fetch = || {
            calls.set(calls.get() + 1);
            Ok(FIXTURE.to_string())
        };
        assert!(load(dir.path(), false, fetch).is_ok());
        assert_eq!(calls.get(), 1); // nothing cached yet
        assert!(load(dir.path(), false, fetch).is_ok());
        assert_eq!(calls.get(), 1); // cache hit, no network
        assert!(load(dir.path(), true, || Err(TreeError::Offline)).is_ok()); // failed refresh keeps the cache
        let empty = tempfile::tempdir().unwrap();
        assert!(matches!(load(empty.path(), false, || Err(TreeError::Offline)), Err(TreeError::Offline)));
        std::fs::write(dir.path().join(CACHE_FILE), "not json").unwrap();
        assert!(load(dir.path(), false, fetch).is_ok()); // corrupt cache is replaced
    }
}
