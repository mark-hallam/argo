//! `argo skill keygen` — generate a cosign-style ed25519 keypair.

use crate::cli::cli_skill_sign_cmds::SkillKeygenCmd;
use argo_skill::sign::{generate_keypair, write_keypair_to_dir};
use argo_types::{ArgoError, ArgoResult};

pub fn run(cmd: SkillKeygenCmd) -> ArgoResult<()> {
    let out_dir = match cmd.out {
        Some(p) => p,
        None => std::env::current_dir()
            .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?,
    };
    let keypair = generate_keypair();
    let files = write_keypair_to_dir(&out_dir, &keypair)?;
    let private = files.private.display().to_string();
    let public = files.public.display().to_string();
    if cmd.json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "command": "keygen",
                "private_key": private,
                "public_key": public,
            }))
            .unwrap_or_default()
        );
    } else {
        println!("wrote cosign-style ed25519 keypair to:");
        println!("  private: {private}");
        println!("  public:  {public}");
        println!("keep the private key secret; share the public key for verify.");
    }
    Ok(())
}
