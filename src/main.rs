use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rbx_questionnaire::api::{resolve_cookie, Api};
use rbx_questionnaire::config::{Config, DEFAULT_FILE};
use rbx_questionnaire::model::{PreviewResponse, Questionnaire};
use rbx_questionnaire::plan::{plan, Plan, Problem};
use rbx_questionnaire::{env, file};

#[derive(Parser)]
#[command(
    name = "rbx-questionnaire",
    version,
    about = "Declare, review and submit a Roblox experience's content questionnaire",
    long_about = "Roblox turns this questionnaire's answers into your experience's age \
                  rating, so the answers live in a file you commit rather than in a flag. \
                  Nothing is defaulted: an unanswered question stops a submission."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write or update rbxquestionnaire.toml from the live questionnaire.
    ///
    /// Only ever appends. An answer already in the file is never rewritten and
    /// never removed, because either would change a declaration filed under
    /// your name without you saying so.
    Pull(Pull),
    /// Print the questions, and what your file answers to each.
    Show(Common),
    /// Exit non-zero unless every question Roblox asks is answered. The CI contract.
    Check(Common),
    /// Validate, show the resulting age rating, and file the declaration.
    Submit(Submit),
}

#[derive(Args)]
struct Common {
    /// The universe to ask about. Takes precedence over --env.
    #[arg(long, value_name = "ID")]
    universe_id: Option<u64>,

    /// An environment name in rbxplace.toml to resolve the universe from.
    #[arg(long, value_name = "NAME")]
    env: Option<String>,

    /// Where rbxplace.toml lives.
    #[arg(long, value_name = "PATH", default_value = env::DEFAULT_FILE)]
    places_file: PathBuf,

    /// The answers file.
    #[arg(long, short = 'c', value_name = "PATH", default_value = DEFAULT_FILE)]
    config: PathBuf,

    /// A .ROBLOSECURITY cookie. Falls back to $RBX_COOKIE, then to a signed-in
    /// local Studio, which is asked for first (see --auto-cookie).
    #[arg(
        long,
        value_name = "COOKIE",
        env = "RBX_COOKIE",
        hide_env_values = true
    )]
    cookie: Option<String>,

    /// Never use the session of a signed-in local Studio.
    #[arg(long)]
    no_auto_cookie: bool,

    /// Use a signed-in local Studio session without asking first.
    #[arg(long, conflicts_with = "no_auto_cookie")]
    auto_cookie: bool,
}

#[derive(Args)]
struct Pull {
    #[command(flatten)]
    common: Common,

    /// Write this answer into every question still blank that offers it,
    /// typically `--fill no`.
    ///
    /// The shortcut for a questionnaire that really is "no" throughout. It is a
    /// flag rather than a default because the answers land in the file, where
    /// they are committed and can be read and disagreed with. An answer nobody
    /// typed is not a declaration; a file somebody generated and kept is.
    /// Existing answers are never overwritten.
    #[arg(long, value_name = "ANSWER")]
    fill: Option<String>,
}

#[derive(Args)]
struct Submit {
    #[command(flatten)]
    common: Common,

    /// Validate and show the rating, then stop without filing anything.
    #[arg(long)]
    dry_run: bool,

    /// Skip the confirmation prompt. For CI, where nobody is there to answer.
    #[arg(long, short = 'y')]
    yes: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Pull(args) => run_pull(args).await,
        Command::Show(args) => run_show(args).await,
        Command::Check(args) => run_check(args).await,
        Command::Submit(args) => run_submit(args).await,
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// Everything a command needs from Roblox, fetched once.
struct Live {
    universe_id: u64,
    questionnaire: Questionnaire,
    api: Api,
}

async fn fetch(common: &Common) -> Result<Live> {
    let universe_id = match (common.universe_id, &common.env) {
        (Some(id), _) => id,
        (None, Some(name)) => env::universe_id(&common.places_file, name)?,
        (None, None) => bail!("pass --universe-id, or --env with an rbxplace.toml"),
    };

    let api = Api::new(resolve_cookie(
        common.cookie.as_deref(),
        common.no_auto_cookie,
        common.auto_cookie,
    )?)?;
    let id = api.latest_id(universe_id).await?;
    let questionnaire = api.questionnaire(&id).await?;

    Ok(Live {
        universe_id,
        questionnaire,
        api,
    })
}

fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        bail!(
            "{} does not exist. Run `rbx-questionnaire pull` to write one from the \
             live questionnaire, then fill in the answers.",
            path.display()
        );
    }
    Config::load(path)
}

async fn run_pull(args: Pull) -> Result<ExitCode> {
    let live = fetch(&args.common).await?;
    let path = &args.common.config;

    let existing = if path.exists() {
        Some(std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?)
    } else {
        None
    };

    let (text, merged) = file::merge(
        existing.as_deref(),
        &live.questionnaire,
        args.fill.as_deref(),
    )?;
    std::fs::write(path, &text).with_context(|| format!("writing {}", path.display()))?;

    let total = live.questionnaire.all_questions().len();
    match (existing.is_some(), merged.added) {
        (false, n) => println!("Wrote {} with {n} question(s).", path.display()),
        (true, 0) => println!("{} is up to date ({total} question(s)).", path.display()),
        (true, n) => println!(
            "Added {n} new question(s) to {}. Nothing already there was touched.",
            path.display()
        ),
    }

    if merged.filled > 0 {
        println!(
            "Answered {} of them with '{}'. Read them before submitting: they are \
             what your game declares.",
            merged.filled,
            args.fill.as_deref().unwrap_or_default()
        );
    }

    if !merged.unfilled.is_empty() {
        // Not a failure. These are the follow-ups whose choices are not the word
        // that was asked for, and they only matter if something above them is
        // answered yes.
        let mut topics = merged.unfilled.clone();
        topics.sort();
        topics.dedup();
        println!("Left blank, no matching option: {}.", topics.join(", "));
    }

    if merged.added > merged.filled {
        println!("Fill in each remaining `answer`, then run `check`.");
    }

    Ok(ExitCode::SUCCESS)
}

async fn run_show(args: Common) -> Result<ExitCode> {
    let live = fetch(&args).await?;
    let config = load_config(&args.config)?;
    let plan = plan(&live.questionnaire, &config);

    println!(
        "Universe {}, questionnaire {}",
        live.universe_id, live.questionnaire.id
    );
    println!();

    print_answers(&plan);

    if !plan.problems.is_empty() {
        println!();
        print_problems(&plan);
    }

    Ok(ExitCode::SUCCESS)
}

async fn run_check(args: Common) -> Result<ExitCode> {
    let live = fetch(&args).await?;
    let asked = questions_asked(&live.questionnaire)?;
    let config = load_config(&args.config)?;
    let plan = plan(&live.questionnaire, &config);

    let blocking = plan.blocking().count();

    if plan.problems.is_empty() {
        println!(
            "{asked} question(s) asked, {} answered.",
            plan.answers.len()
        );
        return Ok(ExitCode::SUCCESS);
    }

    print_problems(&plan);

    if blocking > 0 {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// How many questions Roblox asks, follow-ups included, or an error if none.
///
/// A questionnaire with no question in it is not one whose every question is
/// answered. It is an empty answer from Roblox, or a shape this tool no longer
/// reads, and passing `check` on it would turn a broken read into a green CI.
fn questions_asked(questionnaire: &Questionnaire) -> Result<usize> {
    let asked = questionnaire.all_questions().len();
    if asked == 0 {
        bail!(
            "no question could be read from questionnaire {}, so nothing was checked. \
             Roblox returned none, or returned them in a shape this tool does not read: \
             these endpoints are undocumented and Roblox can change them without notice",
            questionnaire.id
        );
    }
    Ok(asked)
}

async fn run_submit(args: Submit) -> Result<ExitCode> {
    let live = fetch(&args.common).await?;
    let config = load_config(&args.common.config)?;
    let plan = plan(&live.questionnaire, &config);

    if !plan.is_submittable() {
        print_problems(&plan);
        bail!("refusing to submit an incomplete declaration");
    }

    // Roblox's own opinion first: this tool's idea of "complete" is not the one
    // that counts.
    let validation = live
        .api
        .validate(&live.questionnaire.id, &plan.answers)
        .await?;
    if !validation.is_valid {
        for failure in &validation.failures {
            eprintln!("  {failure}");
        }
        bail!("Roblox rejected these answers as invalid");
    }

    let preview = live
        .api
        .preview(live.universe_id, &live.questionnaire.id, &plan.answers)
        .await?;

    println!("About to declare, for universe {}:", live.universe_id);
    print_answers(&plan);

    println!();
    match preview.age_rating() {
        Some(rating) => println!("Resulting age rating: {rating}"),
        None => println!("Roblox did not return a rating for these answers."),
    }

    match after_preview(&preview, args.dry_run, args.yes)? {
        Step::DryRun => {
            println!("\nDry run, nothing filed.");
            return Ok(ExitCode::SUCCESS);
        }
        Step::Ask => {
            println!();
            let confirmed = dialoguer::Confirm::new()
                .with_prompt("File this declaration with Roblox?")
                .default(false)
                .interact()
                .context("reading the confirmation")?;

            if !confirmed {
                println!("Nothing filed.");
                return Ok(ExitCode::SUCCESS);
            }
        }
        Step::File => {}
    }

    let response = live
        .api
        .submit(live.universe_id, &live.questionnaire.id, &plan.answers)
        .await?;

    if response.submission_id.is_empty() {
        println!("Submitted.");
    } else {
        println!("Submitted ({}).", response.submission_id);
    }

    Ok(ExitCode::SUCCESS)
}

/// What `submit` does once Roblox has previewed the answers.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    /// `--dry-run`: stop here, nothing filed.
    DryRun,
    /// Ask before filing.
    Ask,
    /// `--yes`: file without asking.
    File,
}

/// What follows the preview: stop, ask, or file.
///
/// Roblox's preview can mark a response `disableSubmitButton`. What sets the
/// flag is not documented; its name says the Creator Hub would not offer to
/// submit these answers, so this tool does not either. It stops whatever the
/// flags: `--yes` answers this tool's question and not Roblox's objection, and
/// a dry run that passed would promise a filing the real run refuses.
fn after_preview(preview: &PreviewResponse, dry_run: bool, yes: bool) -> Result<Step> {
    if preview.disable_submit_button {
        bail!(
            "Roblox marks these answers as not submittable (`disableSubmitButton` in \
             its preview), so nothing was filed. The questionnaire page of the Creator \
             Hub is the place to see why."
        );
    }

    Ok(match (dry_run, yes) {
        (true, _) => Step::DryRun,
        (false, true) => Step::File,
        (false, false) => Step::Ask,
    })
}

/// The declared answers, one per line, with the topic column sized to fit.
///
/// Sized rather than fixed: "Continuous Media Feed and Autoplay" is a real
/// topic name and a fixed column swallowed the space after it.
fn print_answers(plan: &Plan) {
    let width = plan
        .reached
        .iter()
        .map(|r| r.topic.chars().count())
        .max()
        .unwrap_or(0);

    for reached in &plan.reached {
        let topic = if reached.topic.is_empty() {
            "?"
        } else {
            &reached.topic
        };
        println!("  {topic:<width$}  {}", reached.answer);
    }
}

/// The question itself, without the guidance under it.
///
/// Roblox's question text is markdown: the question, then a definition, a
/// bulleted list of what not to count, sometimes a link. Printing all of it
/// under an error buries the error.
fn first_line(text: &str) -> &str {
    text.trim()
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches('#')
        .trim()
}

fn print_problems(plan: &Plan) {
    for problem in &plan.problems {
        match problem {
            Problem::Unanswered {
                topic,
                text,
                options,
                ..
            } => {
                eprintln!("  unanswered: {topic}");
                eprintln!("      {}", first_line(text));
                if !options.is_empty() {
                    eprintln!("      options: {}", options.join(", "));
                }
            }
            Problem::UnknownAnswer {
                topic,
                answer,
                options,
                ..
            } => {
                eprintln!("  '{answer}' is not an option for: {topic}");
                eprintln!("      options: {}", options.join(", "));
            }
            Problem::AmbiguousAnswer {
                topic,
                answer,
                matches,
                ..
            } => {
                eprintln!("  '{answer}' fits more than one option for: {topic}");
                eprintln!("      matches: {}", matches.join(", "));
                eprintln!("      write enough of it to tell them apart");
            }
            Problem::Stale { id, topic } => {
                eprintln!("  no longer asked: {topic} ({id})");
                eprintln!(
                    "      harmless, but the questionnaire has changed since this was written"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{after_preview, questions_asked, Step};
    use rbx_questionnaire::model::{PreviewResponse, Questionnaire};

    fn questionnaire(body: &str) -> Questionnaire {
        serde_json::from_str(body).expect("a questionnaire body")
    }

    /// The failure this guards: `check` passed on a questionnaire it had read
    /// nothing from, so an empty answer from Roblox read as a complete
    /// declaration.
    #[test]
    fn a_questionnaire_with_no_question_fails_the_check() {
        for body in [
            r#"{"id": "q1"}"#,
            r#"{"id": "q1", "sections": []}"#,
            r#"{"id": "q1", "sections": [{"name": "Violence", "questions": []}]}"#,
            // Questions under a key this tool does not read: parsed, kept nothing.
            r#"{"id": "q1", "sections": [{"name": "Violence", "items": [{"id": "v"}]}]}"#,
        ] {
            let err = questions_asked(&questionnaire(body)).expect_err(body);
            assert!(
                err.to_string().contains("no question could be read"),
                "{body}: {err}"
            );
        }
    }

    #[test]
    fn a_questionnaire_with_questions_counts_them() {
        let one = questionnaire(
            r#"{"id": "q1", "sections": [{"name": "Violence", "questions": [{"id": "v"}]}]}"#,
        );
        assert_eq!(questions_asked(&one).unwrap(), 1);
    }

    /// Parsed from the wire name rather than built from the field, so a rename
    /// that disconnects Roblox's flag from the refusal fails here.
    fn preview(body: &str) -> PreviewResponse {
        serde_json::from_str(body).expect("a preview body")
    }

    /// The failure this guards: the flag was printed and the declaration filed
    /// all the same, without a question under `--yes`.
    #[test]
    fn answers_roblox_marks_not_submittable_are_never_filed() {
        let marked = preview(r#"{"disableSubmitButton": true}"#);

        for (dry_run, yes) in [(false, true), (false, false), (true, false), (true, true)] {
            let err = after_preview(&marked, dry_run, yes)
                .expect_err("a marked response stops whatever the flags");

            assert!(
                err.to_string().contains("nothing was filed"),
                "dry_run={dry_run} yes={yes}: {err}"
            );
        }
    }

    /// False or absent, Roblox's flag changes nothing and the command's own
    /// flags decide.
    #[test]
    fn unmarked_answers_follow_the_flags() {
        for body in [r#"{"disableSubmitButton": false}"#, "{}"] {
            let unmarked = preview(body);

            assert_eq!(after_preview(&unmarked, false, false).unwrap(), Step::Ask);
            assert_eq!(after_preview(&unmarked, false, true).unwrap(), Step::File);
            assert_eq!(after_preview(&unmarked, true, false).unwrap(), Step::DryRun);
            assert_eq!(
                after_preview(&unmarked, true, true).unwrap(),
                Step::DryRun,
                "a dry run files nothing, --yes or not"
            );
        }
    }
}
