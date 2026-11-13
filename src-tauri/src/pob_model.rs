//! PoB XML -> typed build. Tolerant of missing sections and attributes (PoB versions differ);
//! strict about safety: DTDs are rejected, so there is no entity expansion.

use roxmltree::{Document, Node, ParsingOptions};
use serde::Serialize;

#[derive(Debug, PartialEq, Eq)]
pub enum BuildError {
    Xml,
    NotPob,
}

#[derive(Debug, Default, Serialize)]
pub struct Build {
    pub class: String,
    pub ascendancy: String,
    pub level: u32,
    pub bandit: String,
    pub pantheon_major: String,
    pub pantheon_minor: String,
    /// 0-based index into `specs` of the spec PoB had selected
    pub active_spec: usize,
    pub specs: Vec<Spec>,
    pub skill_sets: Vec<SkillSet>,
    pub item_sets: Vec<ItemSet>,
    pub items: Vec<Item>,
    pub notes: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Spec {
    pub title: String,
    pub tree_version: String,
    pub class_id: u32,
    pub ascend_class_id: u32,
    pub nodes: Vec<u32>,
    /// (mastery node, chosen effect)
    pub masteries: Vec<(u32, u32)>,
}

#[derive(Debug, Default, Serialize)]
pub struct SkillSet {
    pub title: String,
    pub groups: Vec<GemGroup>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct GemGroup {
    pub slot: String,
    pub enabled: bool,
    pub gems: Vec<Gem>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Gem {
    pub name: String,
    pub level: u32,
    pub quality: u32,
    pub support: bool,
    pub enabled: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct ItemSet {
    pub title: String,
    /// (slot name, item id) for filled slots only
    pub slots: Vec<(String, u32)>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Item {
    pub id: u32,
    pub rarity: String,
    pub name: String,
    pub base: String,
    /// mod lines, PoB metadata and `{tag}` prefixes removed
    pub mods: Vec<String>,
}

fn num<T: std::str::FromStr + Default>(n: Node, attr: &str) -> T {
    n.attribute(attr).and_then(|v| v.trim().parse().ok()).unwrap_or_default()
}

fn text<'a>(n: Node<'a, '_>, attr: &str) -> String {
    n.attribute(attr).unwrap_or("").to_string()
}

fn child<'a, 'i>(n: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.has_tag_name(tag))
}

pub fn parse(xml: &str) -> Result<Build, BuildError> {
    let opts = ParsingOptions { allow_dtd: false, nodes_limit: 2_000_000, ..Default::default() };
    let doc = Document::parse_with_options(xml, opts).map_err(|_| BuildError::Xml)?;
    let root = doc.root_element();
    if !root.has_tag_name("PathOfBuilding") {
        return Err(BuildError::NotPob);
    }

    let mut b = Build::default();
    if let Some(n) = child(root, "Build") {
        b.class = text(n, "className");
        b.ascendancy = text(n, "ascendClassName");
        b.level = num(n, "level");
        b.bandit = text(n, "bandit");
        b.pantheon_major = text(n, "pantheonMajorGod");
        b.pantheon_minor = text(n, "pantheonMinorGod");
    }
    if let Some(tree) = child(root, "Tree") {
        b.specs = tree.children().filter(|c| c.has_tag_name("Spec")).map(parse_spec).collect();
        let active: usize = num(tree, "activeSpec");
        b.active_spec = active.saturating_sub(1).min(b.specs.len().saturating_sub(1));
    }
    if let Some(skills) = child(root, "Skills") {
        let sets: Vec<_> = skills.children().filter(|c| c.has_tag_name("SkillSet")).collect();
        b.skill_sets = if sets.is_empty() {
            vec![parse_skill_set(skills)] // older PoB: groups sit directly under <Skills>
        } else {
            sets.into_iter().map(parse_skill_set).collect()
        };
    }
    if let Some(items) = child(root, "Items") {
        b.items = items.children().filter(|c| c.has_tag_name("Item")).map(parse_item).collect();
        let sets: Vec<_> = items.children().filter(|c| c.has_tag_name("ItemSet")).collect();
        b.item_sets = if sets.is_empty() {
            vec![parse_item_set(items)]
        } else {
            sets.into_iter().map(parse_item_set).collect()
        };
    }
    if let Some(n) = child(root, "Notes") {
        b.notes = clean_notes(n.text().unwrap_or(""));
    }
    Ok(b)
}

fn parse_spec(n: Node) -> Spec {
    let ids = |attr: &str| -> Vec<u32> {
        n.attribute(attr).unwrap_or("").split(',').filter_map(|v| v.trim().parse().ok()).collect()
    };
    // "{41016,29214},{...}"
    let masteries = n
        .attribute("masteryEffects")
        .unwrap_or("")
        .split('}')
        .filter_map(|p| {
            let (a, b) = p.trim_start_matches([',', ' ', '{']).split_once(',')?;
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        })
        .collect();
    Spec {
        title: text(n, "title"),
        tree_version: text(n, "treeVersion"),
        class_id: num(n, "classId"),
        ascend_class_id: num(n, "ascendClassId"),
        nodes: ids("nodes"),
        masteries,
    }
}

fn parse_skill_set(n: Node) -> SkillSet {
    let groups = n
        .children()
        .filter(|c| c.has_tag_name("Skill"))
        .map(|g| GemGroup {
            slot: text(g, "slot"),
            enabled: g.attribute("enabled") != Some("false"),
            gems: g
                .children()
                .filter(|c| c.has_tag_name("Gem"))
                .map(|gem| Gem {
                    name: text(gem, "nameSpec"),
                    level: num(gem, "level"),
                    quality: num(gem, "quality"),
                    support: gem.attribute("skillId").is_some_and(|s| s.starts_with("Support"))
                        || gem.attribute("gemId").is_some_and(|s| s.contains("SupportGem")),
                    enabled: gem.attribute("enabled") != Some("false"),
                })
                .collect(),
        })
        .collect();
    SkillSet { title: text(n, "title"), groups }
}

fn parse_item_set(n: Node) -> ItemSet {
    ItemSet {
        title: text(n, "title"),
        slots: n
            .children()
            .filter(|c| c.has_tag_name("Slot"))
            .filter_map(|s| {
                let id: u32 = num(s, "itemId");
                (id != 0).then(|| (text(s, "name"), id))
            })
            .collect(),
    }
}

const ITEM_META: &[&str] = &[
    "Rarity:", "Unique ID:", "Item Level:", "LevelReq:", "Quality:", "Sockets:", "Implicits:", "Prefix:", "Suffix:",
    "Variant:", "Selected Variant", "Catalyst", "Radius:", "League:", "Limited to:", "Cluster Jewel", "Crafted:",
    "Armour:", "Evasion:", "Energy Shield:", "ArmourBasePercentile", "EvasionBasePercentile", "EnergyShieldBasePercentile",
    "Requires", "Corrupted", "Mirrored",
];

fn strip_tags(mut s: &str) -> &str {
    while let Some(rest) = s.strip_prefix('{') {
        match rest.split_once('}') {
            Some((_, after)) => s = after,
            None => break,
        }
    }
    s
}

fn parse_item(n: Node) -> Item {
    let mut lines = n.text().unwrap_or("").lines().map(str::trim).filter(|l| !l.is_empty());
    let rarity = lines.next().and_then(|l| l.strip_prefix("Rarity:")).unwrap_or("").trim().to_string();
    let name = lines.next().unwrap_or("").to_string();
    let is_meta = |l: &str| ITEM_META.iter().any(|p| l.starts_with(p));
    let mut rest = lines.peekable();
    // rare and unique items carry the base type on their own line, magic and normal ones don't
    let base = match (rarity.as_str(), rest.peek()) {
        ("RARE" | "UNIQUE", Some(l)) if !is_meta(l) && !l.starts_with('{') => rest.next().unwrap().to_string(),
        _ => String::new(),
    };
    let mods = rest.filter(|l| !is_meta(l)).map(|l| strip_tags(l).trim().to_string()).filter(|l| !l.is_empty()).collect();
    Item { id: num(n, "id"), rarity, name, base, mods }
}

/// PoB colours its notes with `^7` and `^xRRGGBB` codes; strip those and other control characters.
pub fn clean_notes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut it = raw.chars().peekable();
    while let Some(c) = it.next() {
        if c == '^' {
            match it.peek() {
                Some('x') => {
                    it.next();
                    for _ in 0..6 {
                        if it.peek().is_some_and(|h| h.is_ascii_hexdigit()) {
                            it.next();
                        }
                    }
                }
                Some(d) if d.is_ascii_digit() => {
                    it.next();
                }
                _ => out.push(c),
            }
        } else if !c.is_control() || c == '\n' {
            out.push(c);
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> Build {
        let xml = crate::pob_source::import(include_str!("../../fixtures/pob/real1.txt")).unwrap();
        parse(&xml).unwrap()
    }

    #[test]
    fn real_build_header_and_specs() {
        let b = real();
        assert_eq!((b.class.as_str(), b.ascendancy.as_str(), b.level), ("Ranger", "Pathfinder", 96));
        assert_eq!((b.bandit.as_str(), b.pantheon_major.as_str(), b.pantheon_minor.as_str()), ("Alira", "TheBrineKing", "Gruthkul"));
        assert_eq!(b.specs.len(), 7);
        assert_eq!(b.specs[0].title, "1) Level 1-12");
        assert_eq!(b.specs[0].nodes.len(), 14);
        assert_eq!(b.specs[0].nodes[..3], [64111, 41536, 50459]);
        assert_eq!(b.specs[0].masteries, [(41016, 29214)]);
        assert_eq!(b.specs[0].tree_version, "3_29");
        assert_eq!(b.specs[5].nodes.len(), 128);
        assert_eq!(b.active_spec, 5); // "6) ENDGAME SETUP"
    }

    /// Cheap stand-in for a fuzz run: mutated and truncated copies of a real build must never panic.
    #[test]
    fn mutated_real_build_never_panics() {
        let xml = crate::pob_source::import(include_str!("../../fixtures/pob/real1.txt")).unwrap();
        let bytes = xml.as_bytes();
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as usize
        };
        for _ in 0..300 {
            let mut b = bytes.to_vec();
            for _ in 0..1 + next() % 8 {
                let i = next() % b.len();
                b[i] = (next() % 256) as u8;
            }
            b.truncate(1 + next() % b.len());
            let _ = parse(&String::from_utf8_lossy(&b));
        }
    }

    #[test]
    fn real_build_gems() {
        let b = real();
        assert_eq!(b.skill_sets.len(), 7);
        let first = &b.skill_sets[0];
        assert_eq!(first.title, "1) Level 1-12");
        assert_eq!(first.groups.len(), 5);
        let g = &first.groups[0].gems;
        assert_eq!((g[0].name.as_str(), g[0].level, g[0].support), ("Caustic Arrow", 20, false));
        assert_eq!((g[1].name.as_str(), g[1].level, g[1].support), ("Mirage Archer", 11, true));
    }

    #[test]
    fn real_build_items_and_notes() {
        let b = real();
        assert_eq!(b.items.len(), 47);
        assert_eq!(b.item_sets.len(), 7);
        let set = &b.item_sets[0];
        assert!(set.slots.iter().any(|(s, id)| s == "Amulet" && *id == 22));
        assert!(!set.slots.iter().any(|(s, _)| s.starts_with("Jewel"))); // slots without an item are dropped
        let jewel = b.items.iter().find(|i| i.id == 15).unwrap();
        assert_eq!((jewel.rarity.as_str(), jewel.name.as_str(), jewel.base.as_str()), ("RARE", "Oblivion Scar", "Cobalt Jewel"));
        assert_eq!(jewel.mods, ["11% increased Chaos Damage", "+11% to Cold and Lightning Resistances", "19% increased Damage with Poison"]);
        assert!(b.notes.starts_with("LEVELING NOTES"), "{}", &b.notes[..30]);
        assert!(!b.notes.contains("^x") && !b.notes.contains("^7"));
    }

    #[test]
    fn two_more_real_builds_with_cluster_jewels_and_other_stage_titles() {
        let load = |code: &str| parse(&crate::pob_source::import(code).unwrap()).unwrap();
        let witch = load(include_str!("../../fixtures/pob/real2.txt"));
        assert_eq!((witch.class.as_str(), witch.ascendancy.as_str(), witch.level), ("Witch", "Elementalist", 99));
        assert_eq!(witch.specs.len(), 8);
        assert_eq!(witch.skill_sets.len(), 8);
        assert_eq!((witch.item_sets.len(), witch.items.len()), (8, 76));
        assert_eq!(witch.specs[0].title, "1) Act 1-3 (up to firestorm swap)");
        // cluster jewel nodes use ids above 65535
        assert!(witch.specs.iter().any(|s| s.nodes.iter().any(|&n| n > 65535)));

        let scion = load(include_str!("../../fixtures/pob/real3.txt"));
        assert_eq!((scion.class.as_str(), scion.ascendancy.as_str(), scion.level), ("Scion", "Luminary", 86));
        let titles: Vec<_> = scion.specs.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, ["lv 1-23", "lv 24+", "lv 40+ (Merc Swap)", "lv 67+"]);
        assert_eq!(scion.specs[0].nodes.len(), 28);
        assert_eq!((scion.skill_sets.len(), scion.item_sets.len(), scion.items.len()), (4, 4, 87));
    }

    #[test]
    fn minimal_and_old_style_documents_parse() {
        let b = parse("<PathOfBuilding><Build level=\"5\" className=\"Witch\"/></PathOfBuilding>").unwrap();
        assert_eq!((b.class.as_str(), b.level, b.specs.len(), b.skill_sets.len(), b.item_sets.len()), ("Witch", 5, 0, 0, 0));
        let old = parse(
            "<PathOfBuilding><Skills><Skill slot=\"Helmet\"><Gem nameSpec=\"Arc\" level=\"3\" skillId=\"Arc\"/></Skill></Skills>\
             <Items><Item id=\"1\">Rarity: MAGIC\nSturdy Iron Ring of the Wind\n+10 to Dexterity</Item><Slot name=\"Ring 1\" itemId=\"1\"/></Items></PathOfBuilding>",
        )
        .unwrap();
        assert_eq!(old.skill_sets[0].groups[0].slot, "Helmet");
        assert_eq!(old.items[0].base, "");
        assert_eq!(old.items[0].mods, ["+10 to Dexterity"]);
    }

    #[test]
    fn hostile_or_wrong_documents_are_rejected() {
        assert_eq!(parse("<Other/>").unwrap_err(), BuildError::NotPob);
        assert_eq!(parse("<PathOfBuilding>").unwrap_err(), BuildError::Xml);
        assert_eq!(parse("").unwrap_err(), BuildError::Xml);
        let bomb = "<!DOCTYPE lolz [<!ENTITY a \"aaaaaaaaaa\"><!ENTITY b \"&a;&a;&a;&a;\">]><PathOfBuilding><Notes>&b;</Notes></PathOfBuilding>";
        assert_eq!(parse(bomb).unwrap_err(), BuildError::Xml);
    }

    #[test]
    fn notes_lose_colour_codes_but_keep_text_and_carets() {
        assert_eq!(clean_notes("^xD02090Guide^7 by ^x70FF70me^7\u{7}\n2^3 = 8 ^ ok"), "Guide by me\n2 = 8 ^ ok");
    }
}
