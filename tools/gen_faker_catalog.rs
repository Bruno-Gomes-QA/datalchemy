use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    manifest_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Overrides {
    #[serde(default)]
    alias: Vec<AliasOverride>,
}

#[derive(Debug, Deserialize, Clone)]
struct AliasOverride {
    id: String,
    target: String,
    kind: Option<String>,
    locales: Option<Vec<String>>,
    params: Option<Vec<String>>,
}

#[derive(Debug)]
struct FakerDef {
    module: String,
    struct_name: String,
    has_params: bool,
}

#[derive(Debug, Default)]
struct ImplInfo {
    outputs: BTreeSet<String>,
    supports_all: bool,
    locales: BTreeSet<String>,
}

#[derive(Debug)]
struct Entry {
    id: String,
    module: String,
    struct_name: String,
    has_params: bool,
    output_type: String,
    output_kind: OutputKind,
    supports_en: bool,
    supports_pt_br: bool,
}

#[derive(Debug, Clone, Copy)]
enum OutputKind {
    String,
    Str,
    VecString,
    ChronoDuration,
    TimeDuration,
    Other,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("missing repo root")?;

    let metadata = load_metadata(root)?;
    let fake_manifest = metadata
        .packages
        .iter()
        .find(|pkg| pkg.name == "fake")
        .ok_or("fake crate not found in cargo metadata")?
        .manifest_path
        .clone();
    let fake_root = fake_manifest
        .parent()
        .ok_or("missing fake crate root")?
        .to_path_buf();

    let faker_mod = fake_root.join("src/faker/mod.rs");
    let impls_dir = fake_root.join("src/faker/impls");

    let defs = parse_faker_defs(&faker_mod)?;
    let impls = parse_impls(&impls_dir)?;

    let mut entries = Vec::new();
    for def in defs {
        let key = (def.module.clone(), def.struct_name.clone());
        let Some(info) = impls.get(&key) else {
            continue;
        };
        let supports_en = info.supports_all || info.locales.contains("EN");
        let supports_pt_br = info.supports_all || info.locales.contains("PT_BR");
        if !supports_en && !supports_pt_br {
            continue;
        }
        let (output_type, output_kind) = choose_output(info)?;
        let id = format!("faker.{}.raw.{}", def.module, def.struct_name);
        entries.push(Entry {
            id,
            module: def.module,
            struct_name: def.struct_name,
            has_params: def.has_params,
            output_type,
            output_kind,
            supports_en,
            supports_pt_br,
        });
    }

    entries.sort_by(|a, b| a.id.cmp(&b.id));

    let mut support_map: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    for entry in &entries {
        support_map.insert(entry.id.clone(), (entry.supports_en, entry.supports_pt_br));
    }

    let overrides_path =
        root.join("crates/datalchemy-generate/faker_catalog/overrides.toml");
    let overrides = parse_overrides(&overrides_path)?;

    let generated_ids: BTreeSet<String> =
        entries.iter().map(|entry| entry.id.clone()).collect();

    if std::env::var("DEBUG_FAKER_TOOL").is_ok() {
        eprintln!("generated ids: {}", generated_ids.len());
        eprintln!(
            "contains faker.name.raw.Name: {}",
            generated_ids.contains("faker.name.raw.Name")
        );
    }

    for alias in &overrides.alias {
        if !generated_ids.contains(&alias.target) {
            return Err(format!("alias target not found: {}", alias.target).into());
        }
        if let Some((supports_en, supports_pt_br)) = support_map.get(&alias.target) {
            if let Some(locales) = &alias.locales {
                for locale in locales {
                    let supported = match locale.as_str() {
                        "en_US" => *supports_en,
                        "pt_BR" => *supports_pt_br,
                        _ => {
                            return Err(format!(
                                "unsupported locale '{}' in overrides for '{}'",
                                locale, alias.id
                            )
                            .into())
                        }
                    };
                    if !supported {
                        return Err(format!(
                            "alias locale '{}' not supported by target '{}'",
                            locale, alias.target
                        )
                        .into());
                    }
                }
            }
        }
    }

    let mut all_ids = BTreeSet::new();
    for id in &generated_ids {
        if !all_ids.insert(id.clone()) {
            return Err(format!("duplicate id in generated list: {id}").into());
        }
    }
    for alias in &overrides.alias {
        if !all_ids.insert(alias.id.clone()) {
            return Err(format!("duplicate id from overrides: {}", alias.id).into());
        }
    }

    let mut output = String::new();
    writeln!(
        output,
        "// AUTO-GENERATED BY tools/gen_faker_catalog.rs. DO NOT EDIT."
    )?;
    writeln!(output, "use rand::RngCore;")?;
    writeln!(output)?;
    writeln!(output, "use crate::faker_rs::locales::LocaleKey;")?;
    writeln!(output)?;
    writeln!(output, "use crate::generators::GeneratedValue;")?;
    writeln!(output)?;
    writeln!(output, "use fake::Fake;")?;
    writeln!(output)?;

    writeln!(output, "pub struct AliasEntry {{")?;
    writeln!(output, "    pub id: &'static str,")?;
    writeln!(output, "    pub target: &'static str,")?;
    writeln!(output, "    pub locales: &'static [LocaleKey],")?;
    writeln!(output, "}}")?;
    writeln!(output)?;

    write_array(&mut output, "GENERATED_IDS", &generated_ids)?;
    write_array(
        &mut output,
        "PARAMETERIZED_IDS",
        &entries
            .iter()
            .filter(|entry| entry.has_params)
            .map(|entry| entry.id.clone())
            .collect::<BTreeSet<_>>(),
    )?;
    write_array(&mut output, "ALL_IDS", &all_ids)?;

    writeln!(output, "pub const ALIAS_ENTRIES: &[AliasEntry] = &[")?;
    let mut aliases = overrides.alias.clone();
    aliases.sort_by(|a, b| a.id.cmp(&b.id));
    for alias in &aliases {
        let mut locales = alias.locales.clone().unwrap_or_default();
        locales.sort();
        write!(
            output,
            "    AliasEntry {{ id: \"{}\", target: \"{}\", locales: &[",
            alias.id, alias.target
        )?;
        for (idx, locale) in locales.iter().enumerate() {
            if idx > 0 {
                output.push_str(", ");
            }
            let locale_key = locale_key_literal(locale)?;
            write!(output, "{locale_key}")?;
        }
        writeln!(output, "] }},")?;
    }
    writeln!(output, "];")?;
    writeln!(output)?;

    writeln!(
        output,
        "pub fn alias_entry(id: &str) -> Option<&'static AliasEntry> {{"
    )?;
    writeln!(
        output,
        "    ALIAS_ENTRIES.iter().find(|entry| entry.id == id)"
    )?;
    writeln!(output, "}}")?;
    writeln!(output)?;

    writeln!(
        output,
        "pub fn generate_value(id: &str, locale: LocaleKey, rng: &mut dyn RngCore) -> Option<GeneratedValue> {{"
    )?;
    writeln!(output, "    match (id, locale) {{")?;
    for entry in entries.iter().filter(|entry| !entry.has_params) {
        if entry.supports_en {
            let faker_path = format!(
                "fake::faker::{}::en::{}",
                entry.module, entry.struct_name
            );
            writeln!(
                output,
                "        (\"{}\", LocaleKey::EnUs) => {{",
                entry.id
            )?;
            writeln!(
                output,
                "            let value: {} = {}().fake_with_rng(rng);",
                entry.output_type, faker_path
            )?;
            match entry.output_kind {
                OutputKind::String => {
                    writeln!(output, "            Some(GeneratedValue::Text(value))")?;
                }
                OutputKind::Str => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.to_string()))"
                    )?;
                }
                OutputKind::VecString => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.join(\" \")))"
                    )?;
                }
                OutputKind::ChronoDuration => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.num_seconds().to_string()))"
                    )?;
                }
                OutputKind::TimeDuration => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.whole_seconds().to_string()))"
                    )?;
                }
                OutputKind::Other => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.to_string()))"
                    )?;
                }
            }
            writeln!(output, "        }}")?;
        }
        if entry.supports_pt_br {
            let faker_path = format!(
                "fake::faker::{}::pt_br::{}",
                entry.module, entry.struct_name
            );
            writeln!(
                output,
                "        (\"{}\", LocaleKey::PtBr) => {{",
                entry.id
            )?;
            writeln!(
                output,
                "            let value: {} = {}().fake_with_rng(rng);",
                entry.output_type, faker_path
            )?;
            match entry.output_kind {
                OutputKind::String => {
                    writeln!(output, "            Some(GeneratedValue::Text(value))")?;
                }
                OutputKind::Str => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.to_string()))"
                    )?;
                }
                OutputKind::VecString => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.join(\" \")))"
                    )?;
                }
                OutputKind::ChronoDuration => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.num_seconds().to_string()))"
                    )?;
                }
                OutputKind::TimeDuration => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.whole_seconds().to_string()))"
                    )?;
                }
                OutputKind::Other => {
                    writeln!(
                        output,
                        "            Some(GeneratedValue::Text(value.to_string()))"
                    )?;
                }
            }
            writeln!(output, "        }}")?;
        }
    }
    writeln!(output, "        _ => None,")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}")?;

    let output_path =
        root.join("crates/datalchemy-generate/src/faker_rs/catalog_gen.rs");
    fs::write(&output_path, output)?;

    Ok(())
}

fn load_metadata(root: &Path) -> Result<Metadata, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("cargo metadata failed".into());
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout)?;
    Ok(metadata)
}

fn parse_faker_defs(path: &Path) -> Result<Vec<FakerDef>, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let mut defs = Vec::new();
    let mut current_module: Option<String> = None;
    let mut in_def = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub mod ")
            && trimmed.contains('{')
            && !trimmed.contains('$')
        {
            let name = trimmed
                .trim_start_matches("pub mod ")
                .trim_end_matches('{')
                .trim();
            current_module = Some(name.to_string());
        }

        if trimmed.starts_with("def_fakers!") && !trimmed.contains("@m") {
            in_def = true;
            continue;
        }

        if in_def {
            if trimmed.starts_with('}') {
                in_def = false;
                continue;
            }
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            let module = current_module
                .as_ref()
                .ok_or("def_fakers block outside module")?
                .clone();
            let name_part = trimmed
                .split('(')
                .next()
                .unwrap_or("")
                .split('<')
                .next()
                .unwrap_or("")
                .trim();
            if name_part.is_empty() {
                continue;
            }
            let params_part = trimmed
                .split('(')
                .nth(1)
                .unwrap_or("")
                .split(')')
                .next()
                .unwrap_or("")
                .trim();
            let has_params = !params_part.is_empty();
            defs.push(FakerDef {
                module,
                struct_name: name_part.to_string(),
                has_params,
            });
        }
    }

    Ok(defs)
}

fn parse_impls(
    dir: &Path,
) -> Result<BTreeMap<(String, String), ImplInfo>, Box<dyn std::error::Error>> {
    let mut map: BTreeMap<(String, String), ImplInfo> = BTreeMap::new();
    let regex = Regex::new(
        r"impl\s*(?:<[^>]*>\s*)?Dummy<([A-Za-z0-9_]+)(?:<([^>]+)>)?>\s*for\s*([^\s{]+)",
    )?;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let module = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or("invalid impls module name")?
            .to_string();
        let contents = fs::read_to_string(&path)?;
        for caps in regex.captures_iter(&contents) {
            let faker_name = caps.get(1).unwrap().as_str().to_string();
            let locale = caps.get(2).map(|m| m.as_str().trim().to_string());
            let output = caps.get(3).unwrap().as_str().to_string();

            let key = (module.clone(), faker_name);
            let info = map.entry(key).or_default();
            info.outputs.insert(output);
            if locale.as_deref().is_none() || locale.as_deref() == Some("L") {
                info.supports_all = true;
            } else if let Some(locale) = locale {
                info.locales.insert(locale);
            }
        }
    }

    Ok(map)
}

fn parse_overrides(path: &Path) -> Result<Overrides, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let overrides: Overrides = toml::from_str(&contents)?;
    Ok(overrides)
}

fn choose_output(info: &ImplInfo) -> Result<(String, OutputKind), Box<dyn std::error::Error>> {
    if info.outputs.contains("String") {
        return Ok(("String".to_string(), OutputKind::String));
    }
    if info.outputs.contains("&str") {
        return Ok(("&str".to_string(), OutputKind::Str));
    }
    if info.outputs.contains("Vec<String>") {
        return Ok(("Vec<String>".to_string(), OutputKind::VecString));
    }
    if info.outputs.contains("chrono::Duration") {
        return Ok((
            "chrono::Duration".to_string(),
            OutputKind::ChronoDuration,
        ));
    }
    if info.outputs.contains("time::Duration") {
        return Ok((
            "time::Duration".to_string(),
            OutputKind::TimeDuration,
        ));
    }

    let output = info
        .outputs
        .iter()
        .next()
        .ok_or("missing output type")?;
    Ok((normalize_type(output), OutputKind::Other))
}

fn normalize_type(ty: &str) -> String {
    match ty {
        "IpAddr" => "std::net::IpAddr".to_string(),
        "Ipv4Addr" => "std::net::Ipv4Addr".to_string(),
        "Ipv6Addr" => "std::net::Ipv6Addr".to_string(),
        "PathBuf" => "std::path::PathBuf".to_string(),
        other => other.to_string(),
    }
}

fn write_array(
    output: &mut String,
    name: &str,
    values: &BTreeSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(output, "pub const {name}: &[&str] = &[")?;
    for value in values {
        writeln!(output, "    \"{value}\",")?;
    }
    writeln!(output, "];")?;
    writeln!(output)?;
    Ok(())
}

fn locale_key_literal(locale: &str) -> Result<&'static str, Box<dyn std::error::Error>> {
    match locale {
        "en_US" => Ok("LocaleKey::EnUs"),
        "pt_BR" => Ok("LocaleKey::PtBr"),
        _ => Err(format!("unsupported locale in overrides: {locale}").into()),
    }
}
