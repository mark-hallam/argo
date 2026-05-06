//! Vertical unarchive support for `argo vertical`.

use super::vertical_archive::{
    load_installed, normalize_id, open_store, print_json, unarchive_vertical,
};
use argo_types::ArgoResult;
use serde_json::json;

pub async fn run(id: &str, json_out: bool) -> ArgoResult<()> {
    let id = normalize_id(id)?;
    load_installed(id)?;
    let (store, _) = open_store().await?;
    let out = unarchive_vertical(&store, id).await?;
    if json_out {
        print_json(&json!({"ok": true, "vertical_unarchive": out}))
    } else {
        println!("unarchived vertical {id}");
        Ok(())
    }
}
