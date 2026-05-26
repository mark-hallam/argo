use argo_eval::{PitchChannel, PitchEvalConfig, PitchEvalRunner};
use std::path::PathBuf;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("pitch-eval: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> argo_types::ArgoResult<u8> {
    let args = Args::parse(std::env::args().skip(1))?;
    let config = PitchEvalConfig::from_repo_root(".");
    let runner = PitchEvalRunner::load(&config)?;
    let report = runner.run_suite(&args.suite, args.channel)?;
    print!("{}", report.format_text());
    Ok(report.exit_code(args.gate_must_pass))
}

struct Args {
    suite: PathBuf,
    gate_must_pass: bool,
    channel: Option<PitchChannel>,
}

impl Args {
    fn parse(mut args: impl Iterator<Item = String>) -> argo_types::ArgoResult<Self> {
        let mut suite = PathBuf::from("eval/pitch-agent");
        let mut gate_must_pass = false;
        let mut channel = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--suite" => {
                    let Some(value) = args.next() else {
                        return Err(argo_types::ArgoError::Schema(
                            "--suite requires a path".into(),
                        ));
                    };
                    suite = PathBuf::from(value);
                }
                "--gate-must-pass" => gate_must_pass = true,
                "--channel" => {
                    let Some(value) = args.next() else {
                        return Err(argo_types::ArgoError::Schema(
                            "--channel requires a value".into(),
                        ));
                    };
                    channel = Some(parse_channel(&value)?);
                }
                "--help" | "-h" => {
                    println!(
                        "usage: pitch-eval [--suite eval/pitch-agent] [--gate-must-pass] [--channel WebEmbed|Rehearsal|LiveMeeting|Chat]"
                    );
                    return Ok(Self {
                        suite,
                        gate_must_pass,
                        channel,
                    });
                }
                other => {
                    return Err(argo_types::ArgoError::Schema(format!(
                        "unknown argument '{other}'"
                    )));
                }
            }
        }
        Ok(Self {
            suite,
            gate_must_pass,
            channel,
        })
    }
}

fn parse_channel(raw: &str) -> argo_types::ArgoResult<PitchChannel> {
    match raw {
        "Rehearsal" | "rehearsal" => Ok(PitchChannel::Rehearsal),
        "WebEmbed" | "web_embed" | "web-embed" => Ok(PitchChannel::WebEmbed),
        "LiveMeeting" | "live_meeting" | "live-meeting" => Ok(PitchChannel::LiveMeeting),
        "Chat" | "chat" => Ok(PitchChannel::Chat),
        other => Err(argo_types::ArgoError::Schema(format!(
            "unknown pitch channel '{other}'"
        ))),
    }
}
