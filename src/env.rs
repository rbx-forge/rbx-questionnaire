//! Resolving `--env prod` through `rbxplace.toml`.
//!
//! That file belongs to rbx-deploy, and this tool only reads it. Sharing it is the
//! point: an environment name should mean the same universe to every tool in a
//! project, and asking somebody to repeat a universe id into a second file is
//! how the two drift apart.
//!
//! Parsed loosely rather than against a struct: rbx-deploy owns the schema and
//! adds to it, and a key this tool has never heard of should not stop it
//! finding a universe id.

use anyhow::{bail, Context, Result};
use std::path::Path;

pub const DEFAULT_FILE: &str = "rbxplace.toml";

/// The universe id an env name maps to.
pub fn universe_id(path: &Path, env: &str) -> Result<u64> {
    let text = std::fs::read_to_string(path).with_context(|| {
        format!(
            "reading {} to resolve --env {env}. Pass --universe-id instead if this \
             project has no rbxplace.toml",
            path.display()
        )
    })?;

    let doc: toml::Value =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;

    let Some(table) = doc.get(env) else {
        bail!(
            "{} has no environment named '{env}'. It defines: {}",
            path.display(),
            env_names(&doc).join(", ")
        );
    };

    let Some(id) = table.get("universe_id").and_then(toml::Value::as_integer) else {
        bail!(
            "environment '{env}' in {} has no `universe_id`",
            path.display()
        );
    };

    u64::try_from(id).with_context(|| format!("universe_id {id} is not a valid id"))
}

/// Top-level tables that carry a `universe_id`, which is what an env is here.
/// Anything else in the file belongs to another tool.
fn env_names(doc: &toml::Value) -> Vec<String> {
    let Some(table) = doc.as_table() else {
        return Vec::new();
    };

    table
        .iter()
        .filter(|(_, v)| v.get("universe_id").is_some())
        .map(|(k, _)| k.clone())
        .collect()
}
