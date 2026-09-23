use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod jev_experience;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(unix)]
unsafe extern "C" {
    fn geteuid() -> u32;
    fn flock(fd: i32, operation: i32) -> i32;
}

const LAYERS: [&str; 3] = ["authority", "direct_evidence", "background_experience"];
const TASK_CONTEXT_LAYERS: [&str; 4] = [
    "task_core",
    "current_state",
    "direct_evidence",
    "background_experience",
];
const MAX_JEV_DECISIONS_PER_SYNC: usize = 1;

#[derive(Debug, Deserialize)]
struct Routing {
    #[serde(default)]
    excluded: Vec<Excluded>,
    #[serde(default)]
    knowledge_sources: Vec<KnowledgeSource>,
}

#[derive(Debug, Deserialize)]
struct Excluded {
    path: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct KnowledgeSource {
    id: String,
    kind: String,
    path: String,
    #[serde(default)]
    authority: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RoutePlan {
    version: u32,
    intent: String,
    confidence: f32,
    matched_signals: Vec<String>,
    retrieval_order: Vec<String>,
    source_adapter: String,
    search_policy: SearchPolicy,
    scope: String,
    evidence: Vec<RetrievalEvidence>,
    fallback: String,
}

#[derive(Debug, Serialize)]
struct SearchPolicy {
    first_pass: String,
    semantic_fallback: String,
    max_results: usize,
}

#[derive(Debug, Serialize)]
struct RetrievalEvidence {
    source_id: String,
    path: String,
    match_type: String,
    score: usize,
    authority: String,
    scope_match: bool,
    selected: bool,
    reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Registry {
    version: u32,
    #[serde(default)]
    tasks: Vec<RegistryTask>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RegistryTask {
    task_id: String,
    status: String,
    task_path: String,
    pack_path: String,
    #[serde(default)]
    projects: Vec<String>,
    #[serde(default)]
    preferred_runtimes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Task {
    task_id: String,
    #[serde(default)]
    profile: String,
    goal: String,
    scope: serde_yaml::Value,
    #[serde(default)]
    excluded_scope: Vec<String>,
    stop_condition: String,
    #[serde(default)]
    current_state: String,
    #[serde(default)]
    verified_facts: Vec<String>,
    #[serde(default)]
    assumptions: Vec<String>,
    #[serde(default)]
    decisions: Vec<String>,
    #[serde(default)]
    open_questions: Vec<String>,
    #[serde(default)]
    blockers: Vec<String>,
    #[serde(default)]
    next_actions: Vec<String>,
    #[serde(default)]
    context_sources: Vec<ContextSource>,
    context_policy: ContextPolicy,
}

#[derive(Debug, Deserialize)]
struct ContextSource {
    path: String,
    layer: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ContextPolicy {
    layers: Vec<String>,
    max_items: usize,
    max_chars: usize,
}

#[derive(Debug, Serialize)]
struct AutoTaskFile {
    task_id: String,
    profile: String,
    goal: String,
    scope: AutoTaskScope,
    excluded_scope: Vec<String>,
    stop_condition: String,
    current_state: String,
    context_policy: ContextPolicyOutput,
}

#[derive(Debug, Serialize)]
struct AutoTaskScope {
    projects: Vec<String>,
    services: Vec<String>,
    repositories: Vec<String>,
    environments: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ContextPolicyOutput {
    layers: Vec<String>,
    max_items: usize,
    max_chars: usize,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct TaskState {
    version: u32,
    task_id: String,
    updated_at: String,
    #[serde(default)]
    current_state: String,
    #[serde(default)]
    completed: Vec<String>,
    #[serde(default)]
    next_actions: Vec<String>,
    #[serde(default)]
    blockers: Vec<String>,
    #[serde(default)]
    decisions: Vec<String>,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservationEvent {
    version: u32,
    observation_id: String,
    task_id: String,
    created_at: String,
    runtime: String,
    kind: String,
    scope: Vec<String>,
    source: String,
    summary: String,
    hypothesis: String,
    evidence: Vec<String>,
    status: String,
    confidence: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CandidateRecord {
    version: u32,
    candidate_id: String,
    status: String,
    confidence: String,
    task_id: String,
    observation_ids: Vec<String>,
    kinds: Vec<String>,
    counterexample_observation_ids: Vec<String>,
    sources: Vec<String>,
    source: String,
    observation: String,
    hypothesis: String,
    scope: Vec<String>,
    promotion_condition: String,
    next_action: String,
    #[serde(default)]
    reviewed_at: String,
    #[serde(default)]
    review_reason: String,
    #[serde(default)]
    jev_review_observation_count: usize,
    #[serde(default)]
    decision_history: Vec<DecisionRecord>,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    feedback_history: Vec<FeedbackReference>,
    #[serde(default)]
    identity: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DecisionRecord {
    decision: String,
    at: String,
    reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FeedbackReference {
    feedback_id: String,
    observation_id: String,
    action: String,
    at: String,
    summary: String,
}

#[derive(Debug, Serialize)]
struct ReviewQueue {
    version: u32,
    task_id: String,
    generated_at: String,
    human_gate: String,
    candidates: Vec<ReviewItem>,
}

#[derive(Debug, Serialize)]
struct ReviewItem {
    candidate_id: String,
    observation_count: usize,
    source_count: usize,
    kinds: Vec<String>,
    counterexample_count: usize,
    action: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ExperienceBundle {
    version: u32,
    task_id: String,
    runtime: String,
    generated_at: String,
    source: String,
    experiences: Vec<ConsumedExperience>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PortableExperiencePack {
    version: u32,
    kind: String,
    created_at: String,
    source: String,
    source_roots: Vec<String>,
    candidates: Vec<CandidateRecord>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConsumedExperience {
    candidate_id: String,
    owner: String,
    observation: String,
    hypothesis: String,
    scope: Vec<String>,
    sources: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FeedbackRecord {
    version: u32,
    feedback_id: String,
    candidate_id: String,
    task_id: String,
    runtime: String,
    kind: String,
    action: String,
    summary: String,
    observation_id: String,
    previous_status: String,
    resulting_status: String,
    created_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FeedbackJournal {
    version: u32,
    feedback_id: String,
    candidate_path: String,
    original_candidate_yaml: String,
    observation_path: String,
    feedback_path: String,
    #[serde(default)]
    owner_pid: u32,
    #[serde(default)]
    created_at: String,
    committed: bool,
}

#[derive(Debug, Deserialize)]
struct ExecutionPolicies {
    version: u32,
    default_profile: String,
    profiles: BTreeMap<String, ExecutionProfile>,
}

#[derive(Debug, Deserialize)]
struct ExecutionProfile {
    effort: String,
    model: String,
    verification: String,
    human_gate: String,
    allowed_actions: Vec<String>,
    skills: Vec<String>,
    tools: Vec<String>,
    delegation: String,
    #[serde(default)]
    notes: String,
}

#[derive(Debug, Serialize)]
struct Handoff {
    handoff_id: String,
    task_id: String,
    reason: String,
    summary: String,
    goal: String,
    scope: serde_yaml::Value,
    excluded_scope: Vec<String>,
    verified_facts: Vec<String>,
    assumptions: Vec<String>,
    decisions: Vec<String>,
    completed: Vec<String>,
    blockers: Vec<String>,
    next_actions: Vec<String>,
    artifacts: Vec<String>,
    contamination_signals: Vec<String>,
    stop_condition: String,
    do_not_reuse: Vec<String>,
}

struct Harness {
    root: PathBuf,
    routing: Routing,
    registry: Registry,
}

struct OrganizationReport {
    candidate_count: usize,
    observation_count: usize,
    rejected_count: usize,
    paths: Vec<PathBuf>,
}

struct CandidateLock {
    #[cfg(unix)]
    _file: std::fs::File,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("agent-context-harness: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().ok_or_else(usage)?;
    let routing_override = option_path(&args, "--routing");
    let root = resolve_workspace_root(&args, routing_override.as_deref())?;
    let mut harness = Harness::load(root, routing_override)?;
    let task_hint = option_value(&args, "--task")
        .or_else(|| env_setting("AGENT_TASK_ID", "MIYAGO_TASK_ID"))
        .or_else(|| positional_task(&args));
    let cwd = option_path(&args, "--cwd")
        .unwrap_or(env::current_dir().map_err(|e| format!("cannot read cwd: {e}"))?);

    if task_hint.is_none() && command_creates_local_task(command) {
        harness.ensure_local_task(&cwd)?;
    }

    match command.as_str() {
        "route" => {
            let query = required_option(&args, "--query", "route")?;
            let selected = harness.discover(task_hint.as_deref(), &cwd, true).ok();
            let scope = selected
                .map(|task| task.task_id.clone())
                .unwrap_or_else(|| cwd.display().to_string());
            let plan = harness.build_route_plan(&query, &cwd, &scope)?;
            let output = serde_yaml::to_string(&plan)
                .map_err(|e| format!("cannot serialize route plan: {e}"))?;
            if let Some(path) = option_path(&args, "--output") {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("cannot create route output directory: {e}"))?;
                }
                fs::write(&path, output)
                    .map_err(|e| format!("cannot write route plan {}: {e}", path.display()))?;
                println!("route_plan_path: {}", path.display());
            } else {
                print!("{output}");
            }
        }
        "discover" => {
            let cwd = option_path(&args, "--cwd")
                .unwrap_or(env::current_dir().map_err(|e| format!("cannot read cwd: {e}"))?);
            let selected = harness.discover(task_hint.as_deref(), &cwd, false)?;
            println!("task_id: {}", selected.task_id);
            println!(
                "task_path: {}",
                harness.root.join(&selected.task_path).display()
            );
            println!(
                "pack_path: {}",
                harness.root.join(&selected.pack_path).display()
            );
            println!("scope_projects: {}", selected.projects.join(", "));
            println!(
                "preferred_runtimes: {}",
                selected.preferred_runtimes.join(", ")
            );
            println!("session_action: continue");
        }
        "assemble" | "resume" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let mut pack = harness.assemble(&task)?;
            if let Some(query) = option_value(&args, "--query") {
                let route = harness.build_route_plan(&query, &cwd, &selected.task_id)?;
                let route_yaml = serde_yaml::to_string(&route)
                    .map_err(|e| format!("cannot serialize route plan: {e}"))?;
                pack.push_str("\n\n## Retrieval Route\n\n~~~yaml\n");
                pack.push_str(&route_yaml);
                pack.push_str("~~~\n");
            }
            let pack_path = harness.root.join(&selected.pack_path);
            if let Some(parent) = pack_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("cannot create pack directory: {e}"))?;
            }
            fs::write(&pack_path, pack).map_err(|e| format!("cannot write pack: {e}"))?;
            println!("task_id: {}", selected.task_id);
            println!("pack_path: {}", pack_path.display());
            if option_value(&args, "--query").is_some() {
                println!("route_plan: applied");
            }
            println!("session_action: continue");
            if command == "resume" {
                println!();
                println!("# Resume Task: {}", selected.task_id);
                println!();
                println!("請先讀取以下 Context Pack，將它視為本次任務的交接上下文：");
                println!();
                println!("{}", pack_path.display());
                println!();
                println!("請遵守其中的 goal、scope、excluded_scope、decisions 與 next_actions。");
                println!("不要讀取被明確排除的來源。先摘要目前狀態，再開始執行。");
            }
        }
        "sync" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let runtime = option_value(&args, "--runtime")
                .or_else(|| env_setting("AGENT_RUNTIME", "MIYAGO_RUNTIME"))
                .ok_or("sync requires --runtime RUNTIME or AGENT_RUNTIME".to_string())?;
            let experience_task_id = option_value(&args, "--experience-task")
                .or_else(|| env_setting("AGENT_EXPERIENCE_TASK_ID", "MIYAGO_EXPERIENCE_TASK_ID"))
                .ok_or(
                    "sync requires --experience-task ID or AGENT_EXPERIENCE_TASK_ID".to_string(),
                )?;
            let experience_selected = harness.discover(Some(&experience_task_id), &cwd, true)?;
            let experience_task = harness.read_task(&experience_selected)?;
            let report = harness.organize_observations(&experience_task, &experience_selected)?;
            let mut auto_confirmed = 0usize;
            let mut jev_kept = 0usize;
            let mut jev_discarded = 0usize;
            let mut jev_review = 0usize;
            let mut jev_skipped = 0usize;
            let mut jev_failed = 0usize;
            let mut jev_calls = 0usize;
            let jev_enabled = matches!(
                env::var("JEV_EXPERIENCE_RETENTION").as_deref(),
                Ok("1" | "on" | "true")
            );
            let jev_api_key = env::var("TYPESAFE_API_KEY").ok();
            let mut jev_processed = 0usize;
            let jev_threshold = env::var("JEV_EXPERIENCE_MIN_CONFIDENCE")
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or_else(|| {
                    if env::var_os("JEV_EXPERIENCE_MIN_CONFIDENCE").is_some() {
                        -1.0
                    } else {
                        0.75
                    }
                });
            for path in &report.paths {
                let candidate: CandidateRecord = read_yaml(path)?;
                if candidate.status == "candidate"
                    && candidate.counterexample_observation_ids.is_empty()
                    && candidate
                        .kinds
                        .iter()
                        .all(|kind| kind == "explicit_preference")
                    && candidate.sources.iter().all(|source| {
                        source.starts_with("user_message") || source.starts_with("runtime_hook")
                    })
                {
                    harness.decide_candidate(
                        &experience_task,
                        &candidate.candidate_id,
                        "confirm",
                        "明確個人偏好由 runtime sync 自動確認",
                        "personal_model",
                    )?;
                    auto_confirmed += 1;
                    continue;
                }
                if !jev_enabled || candidate.status != "candidate" {
                    continue;
                }
                if !candidate.counterexample_observation_ids.is_empty()
                    || jev_review_is_current(
                        &candidate.decision_history,
                        candidate.jev_review_observation_count,
                        candidate.observation_ids.len(),
                    )
                {
                    jev_skipped += 1;
                    continue;
                }
                if jev_calls >= MAX_JEV_DECISIONS_PER_SYNC {
                    jev_skipped += 1;
                    continue;
                }
                let Some(api_key) = jev_api_key.as_deref().filter(|key| !key.trim().is_empty())
                else {
                    jev_skipped += 1;
                    continue;
                };
                jev_calls += 1;
                match jev_experience::evaluate(
                    &candidate.kinds,
                    &candidate.observation,
                    candidate.observation_ids.len(),
                    Some(api_key),
                    jev_threshold,
                ) {
                    Ok(assessment) => {
                        jev_processed += 1;
                        let reason = format!(
                            "jev:{};choice={};model={};confidence={:.3};minimum={:.2};p_keep={:.3};p_discard={:.3};p_review={:.3}",
                            assessment.decision.as_str(),
                            assessment.choice,
                            assessment.model,
                            assessment.confidence,
                            jev_threshold,
                            assessment.probabilities[0],
                            assessment.probabilities[1],
                            assessment.probabilities[2]
                        );
                        match assessment.decision {
                            jev_experience::RetentionDecision::Keep => {
                                harness.decide_candidate(
                                    &experience_task,
                                    &candidate.candidate_id,
                                    "confirm",
                                    &reason,
                                    "experience_library",
                                )?;
                                jev_kept += 1;
                            }
                            jev_experience::RetentionDecision::Discard => {
                                harness.decide_candidate(
                                    &experience_task,
                                    &candidate.candidate_id,
                                    "reject",
                                    &reason,
                                    "",
                                )?;
                                jev_discarded += 1;
                            }
                            jev_experience::RetentionDecision::Review => {
                                harness.record_jev_review(
                                    &experience_task,
                                    &candidate.candidate_id,
                                    &reason,
                                )?;
                                jev_review += 1;
                            }
                        }
                    }
                    Err(jev_experience::JevError::UnsafeCandidate) => {
                        let reason = "jev:review;reason=local_privacy_filter";
                        harness.record_jev_review(
                            &experience_task,
                            &candidate.candidate_id,
                            reason,
                        )?;
                        jev_review += 1;
                    }
                    Err(jev_experience::JevError::MissingKey) => {
                        jev_skipped += 1;
                    }
                    Err(error) => {
                        eprintln!("experience sync: {}", error.message());
                        jev_failed += 1;
                    }
                }
            }
            let review_path = harness.build_review_queue(&experience_task)?;
            let bundle = harness.consume_confirmed(
                &experience_task,
                &runtime,
                None,
                Some(&selected.projects),
            )?;
            println!("task_id: {}", task.task_id);
            println!("runtime: {}", runtime);
            println!("auto_confirmed: {}", auto_confirmed);
            println!(
                "jev_status: {}",
                if jev_enabled { "enabled" } else { "disabled" }
            );
            println!("jev_processed: {}", jev_processed);
            println!("jev_kept: {}", jev_kept);
            println!("jev_discarded: {}", jev_discarded);
            println!("jev_review: {}", jev_review);
            println!("jev_skipped: {}", jev_skipped);
            println!("jev_failed: {}", jev_failed);
            println!("review_queue_path: {}", review_path.display());
            println!("experience_bundle_path: {}", bundle.display());
            println!("session_action: continue");
        }
        "export" => {
            let output = required_path_option(&args, "--output", "export")?;
            let (path, count) = harness.export_experience_pack(&output)?;
            println!("pack_path: {}", path.display());
            println!("confirmed_count: {}", count);
            println!("raw_transcript: none");
        }
        "import" => {
            let input = required_path_option(&args, "--input", "import")?;
            let mappings = parse_path_mappings(&args)?;
            let (imported, skipped) = harness.import_experience_pack(&input, &mappings)?;
            println!("pack_path: {}", input.display());
            println!("imported_count: {}", imported);
            println!("skipped_count: {}", skipped);
            println!("raw_transcript: none");
        }
        "status" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, false)?;
            let task = harness.read_task(&selected)?;
            println!("task_id: {}", task.task_id);
            println!("profile: {}", task.profile);
            let state = harness.read_state(&task.task_id)?;
            println!(
                "current_state: {}",
                effective_text(state.as_ref(), &task.current_state)
            );
            println!(
                "blockers: {}",
                effective_list(state.as_ref(), StateField::Blockers, &task.blockers).join(" | ")
            );
            println!(
                "next_actions: {}",
                effective_list(state.as_ref(), StateField::NextActions, &task.next_actions)
                    .join(" | ")
            );
            println!("stop_condition: {}", task.stop_condition);
        }
        "observe" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let kind = required_option(&args, "--kind", "observe")?;
            let runtime = option_value(&args, "--runtime")
                .or_else(|| env_setting("AGENT_RUNTIME", "MIYAGO_RUNTIME"))
                .ok_or("observe requires --runtime RUNTIME or AGENT_RUNTIME".to_string())?;
            let source = required_option(&args, "--source", "observe")?;
            let summary = required_option(&args, "--summary", "observe")?;
            let hypothesis = option_value(&args, "--hypothesis").unwrap_or_default();
            let scopes = option_values(&args, "--scope");
            let scopes = if scopes.is_empty() {
                vec![cwd.display().to_string()]
            } else {
                scopes
            };
            let evidence = option_values(&args, "--evidence");
            let event = harness.make_observation(
                &task, &selected, runtime, kind, scopes, source, summary, hypothesis, evidence,
            )?;
            let path = harness.write_observation(&event)?;
            println!("task_id: {}", task.task_id);
            println!("observation_id: {}", event.observation_id);
            println!("observation_path: {}", path.display());
            println!("status: candidate");
            println!("session_action: continue");
        }
        "organize" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let report = harness.organize_observations(&task, &selected)?;
            println!("task_id: {}", task.task_id);
            println!("candidate_count: {}", report.candidate_count);
            println!("observation_count: {}", report.observation_count);
            println!("rejected_count: {}", report.rejected_count);
            for path in report.paths {
                println!("candidate_path: {}", path.display());
            }
            println!("status: candidate_only");
            println!("session_action: continue");
        }
        "review" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let path = harness.build_review_queue(&task)?;
            println!("task_id: {}", task.task_id);
            println!("review_queue_path: {}", path.display());
            println!("human_gate: required");
            println!("session_action: continue");
        }
        "decide" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let decision = required_option(&args, "--decision", "decide")?;
            let reason = option_value(&args, "--reason").unwrap_or_default();
            let owner =
                option_value(&args, "--owner").unwrap_or_else(|| "personal_model".to_string());
            let candidate_id = harness.resolve_decision_candidate(
                &task,
                option_value(&args, "--candidate").as_deref(),
                &decision,
            )?;
            let path =
                harness.decide_candidate(&task, &candidate_id, &decision, &reason, &owner)?;
            println!("task_id: {}", task.task_id);
            println!("candidate_path: {}", path.display());
            println!("decision: {}", decision);
            println!("writeback: none");
            println!("session_action: continue");
        }
        "consume" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let runtime = required_option(&args, "--runtime", "consume")?;
            let candidate_id = option_value(&args, "--candidate");
            let path = harness.consume_confirmed(&task, &runtime, candidate_id.as_deref(), None)?;
            println!("task_id: {}", task.task_id);
            println!("runtime: {}", runtime);
            println!("experience_bundle_path: {}", path.display());
            println!("writeback: none");
            println!("session_action: continue");
        }
        "feedback" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let candidate_id = required_option(&args, "--candidate", "feedback")?;
            let runtime = required_option(&args, "--runtime", "feedback")?;
            let kind = required_option(&args, "--kind", "feedback")?;
            let action = required_option(&args, "--action", "feedback")?;
            let summary = required_option(&args, "--summary", "feedback")?;
            let scopes = option_values(&args, "--scope");
            let source =
                option_value(&args, "--source").unwrap_or_else(|| "user_message".to_string());
            let (path, status) = harness.record_feedback(
                &task,
                &selected,
                &candidate_id,
                &runtime,
                &kind,
                &action,
                &summary,
                scopes,
                &source,
            )?;
            println!("task_id: {}", task.task_id);
            println!("feedback_path: {}", path.display());
            println!("resulting_status: {}", status);
            println!("session_action: continue");
        }
        "checkpoint" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, false)?;
            let task = harness.read_task(&selected)?;
            let mut state = harness
                .read_state(&task.task_id)?
                .unwrap_or_else(|| TaskState {
                    version: 1,
                    task_id: task.task_id.clone(),
                    updated_at: String::new(),
                    ..TaskState::default()
                });
            if let Some(summary) = option_value(&args, "--summary") {
                state.current_state = summary;
            }
            append_unique(&mut state.completed, option_values(&args, "--completed"));
            append_unique(&mut state.next_actions, option_values(&args, "--next"));
            append_unique(&mut state.blockers, option_values(&args, "--blocker"));
            append_unique(&mut state.decisions, option_values(&args, "--decision"));
            append_unique(&mut state.evidence, option_values(&args, "--evidence"));
            state.updated_at = unix_timestamp();
            harness.write_state(&state)?;
            println!("task_id: {}", task.task_id);
            println!(
                "state_path: {}",
                harness.state_path(&task.task_id).display()
            );
            let runtime = option_value(&args, "--runtime")
                .or_else(|| env_setting("AGENT_RUNTIME", "MIYAGO_RUNTIME"))
                .unwrap_or_else(|| "codex".to_string());
            match harness
                .make_observation(
                    &task,
                    &selected,
                    runtime,
                    "checkpoint".to_string(),
                    selected.projects.clone(),
                    "checkpoint".to_string(),
                    effective_text(Some(&state), &task.current_state),
                    String::new(),
                    vec![harness.state_path(&task.task_id).display().to_string()],
                )
                .and_then(|event| {
                    let path = harness.write_observation(&event)?;
                    Ok((event.observation_id, path))
                }) {
                Ok((observation_id, path)) => {
                    println!("observation_id: {}", observation_id);
                    println!("observation_path: {}", path.display());
                    println!("observation_status: candidate");
                }
                Err(error) => println!("observation_status: skipped ({error})"),
            }
            println!("session_action: continue");
        }
        "plan" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, true)?;
            let task = harness.read_task(&selected)?;
            let policies = load_execution_policies(&harness.root)?;
            if policies.version != 1 {
                return Err(format!(
                    "unsupported execution policy version: {}",
                    policies.version
                ));
            }
            let profile_name = if policies.profiles.contains_key(&task.profile) {
                task.profile.clone()
            } else {
                policies.default_profile.clone()
            };
            let profile = policies
                .profiles
                .get(&profile_name)
                .ok_or_else(|| format!("execution policy profile not found: {profile_name}"))?;
            println!("task_id: {}", task.task_id);
            println!("profile: {}", profile_name);
            println!("effort: {}", profile.effort);
            println!("model: {}", profile.model);
            println!("skills: {}", profile.skills.join(", "));
            println!("tools: {}", profile.tools.join(", "));
            println!("delegation: {}", profile.delegation);
            println!("verification: {}", profile.verification);
            println!("human_gate: {}", profile.human_gate);
            println!("allowed_actions: {}", profile.allowed_actions.join(" | "));
            println!("notes: {}", profile.notes);
            println!(
                "preferred_runtimes: {}",
                selected.preferred_runtimes.join(", ")
            );
        }
        "handoff" => {
            let selected = harness.discover(task_hint.as_deref(), &cwd, false)?;
            let task = harness.read_task(&selected)?;
            let reason = option_value(&args, "--reason")
                .ok_or("handoff requires --reason REASON".to_string())?;
            let state = harness.read_state(&task.task_id)?;
            let handoff = harness.make_handoff(&task, &selected, reason.clone(), state.as_ref())?;
            let path = harness
                .root
                .join("records/handoffs")
                .join(format!("{}-handoff.yaml", task.task_id));
            fs::create_dir_all(path.parent().unwrap())
                .map_err(|e| format!("cannot create handoff directory: {e}"))?;
            let yaml = serde_yaml::to_string(&handoff)
                .map_err(|e| format!("cannot serialize handoff: {e}"))?;
            fs::write(&path, yaml).map_err(|e| format!("cannot write handoff: {e}"))?;
            println!("task_id: {}", task.task_id);
            println!("handoff_path: {}", path.display());
            let runtime = option_value(&args, "--runtime")
                .or_else(|| env_setting("AGENT_RUNTIME", "MIYAGO_RUNTIME"))
                .unwrap_or_else(|| "codex".to_string());
            match harness
                .make_observation(
                    &task,
                    &selected,
                    runtime,
                    "handoff".to_string(),
                    selected.projects.clone(),
                    "handoff".to_string(),
                    reason,
                    String::new(),
                    vec![path.display().to_string()],
                )
                .and_then(|event| {
                    let path = harness.write_observation(&event)?;
                    Ok((event.observation_id, path))
                }) {
                Ok((observation_id, observation_path)) => {
                    println!("observation_id: {}", observation_id);
                    println!("observation_path: {}", observation_path.display());
                    println!("observation_status: candidate");
                }
                Err(error) => println!("observation_status: skipped ({error})"),
            }
            println!("session_action: handoff");
        }
        _ => return Err(usage()),
    }
    Ok(())
}

fn usage() -> String {
    "usage: context-harness <discover|route|assemble|resume|sync|export|import|status|observe|organize|review|decide|consume|feedback|checkpoint|plan|handoff> [TASK_ID] [--task ID] [--cwd PATH] [--workspace-root PATH] [--routing PATH] [--query TEXT] [--kind KIND] [--runtime RUNTIME] [--experience-task ID] [--scope PATH] [--source SOURCE] [--summary TEXT] [--hypothesis TEXT] [--evidence PATH] [--candidate ID] [--decision DECISION] [--action ACTION] [--owner OWNER] [--reason TEXT] [--output PATH] [--input PATH] [--map OLD=NEW]\n\n--cwd is the project scope; routing.yaml is loaded from the canonical Context Harness workspace. route creates a provider-neutral source selection and retrieval evidence plan; its first backend is local files and its semantic fallback is intentionally disabled. sync organizes observations, auto-confirms explicit user preferences, optionally asks Jev to classify safe summaries, and emits a scope-filtered bundle; uncertain candidates remain in the review gate. export/import move confirmed summaries only; import accepts explicit scope mappings. decide may omit --candidate when exactly one candidate is eligible; consume without --candidate bundles all confirmed experiences for the task.".to_string()
}

fn required_option(args: &[String], name: &str, command: &str) -> Result<String, String> {
    option_value(args, name).ok_or_else(|| format!("{command} requires {name} VALUE"))
}

fn required_path_option(args: &[String], name: &str, command: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(required_option(args, name, command)?))
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn option_values(args: &[String], name: &str) -> Vec<String> {
    args.windows(2)
        .filter(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .collect()
}

fn parse_path_mappings(args: &[String]) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut mappings = Vec::new();
    for value in option_values(args, "--map") {
        let (old, new) = value
            .split_once('=')
            .ok_or_else(|| "--map requires OLD=NEW".to_string())?;
        let old = PathBuf::from(old);
        let new = PathBuf::from(new);
        if !old.is_absolute() || !new.is_absolute() {
            return Err("--map paths must be absolute".to_string());
        }
        mappings.push((old, new));
    }
    mappings.sort_by(|left, right| {
        right
            .0
            .to_string_lossy()
            .len()
            .cmp(&left.0.to_string_lossy().len())
    });
    Ok(mappings)
}

fn option_path(args: &[String], name: &str) -> Option<PathBuf> {
    option_value(args, name).map(PathBuf::from)
}

fn resolve_workspace_root(args: &[String], routing: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(root) = option_path(args, "--workspace-root") {
        return Ok(root);
    }
    if let Some(routing) = routing {
        let parent = routing
            .parent()
            .ok_or_else(|| "--routing must have a parent directory".to_string())?;
        return Ok(parent.to_path_buf());
    }
    env_setting("AGENT_CONTEXT_WORKSPACE_ROOT", "MIYAGO_CONTEXT_HARNESS_ROOT")
        .or_else(|| env::var("AGENT_FACTORY_WORKSPACE_ROOT").ok())
        .or_else(|| env::var("MIYAGO_AGENT_WORKSPACE_ROOT").ok())
        .map(PathBuf::from)
        .ok_or_else(|| {
            "cannot resolve canonical workspace: set --workspace-root, --routing, or AGENT_CONTEXT_WORKSPACE_ROOT"
                .to_string()
        })
}

fn env_setting(primary: &str, legacy: &str) -> Option<String> {
    env::var(primary).ok().or_else(|| env::var(legacy).ok())
}

fn command_creates_local_task(command: &str) -> bool {
    matches!(
        command,
        "discover"
            | "session-start"
            | "bootstrap"
            | "sync"
            | "assemble"
            | "resume"
            | "plan"
            | "checkpoint"
            | "handoff"
            | "observe"
            | "organize"
            | "review"
            | "decide"
            | "consume"
            | "feedback"
    )
}

fn project_root(cwd: &Path) -> Result<PathBuf, String> {
    let cwd = canonical_existing_path(cwd, "cwd")?;
    let mut current = cwd.as_path();
    loop {
        let git = current.join(".git");
        if git.is_dir() || git.is_file() {
            return Ok(current.to_path_buf());
        }
        current = current
            .parent()
            .ok_or_else(|| format!("cannot find project root for {}", cwd.display()))?;
    }
}

fn auto_task_id(project: &Path) -> String {
    let name = project
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("workspace");
    let mut slug = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '-' {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    format!(
        "auto-{}-{}",
        if slug.is_empty() { "workspace" } else { slug },
        unix_timestamp()
    )
}

fn positional_task(args: &[String]) -> Option<String> {
    args.get(1).filter(|arg| !arg.starts_with("--")).cloned()
}

impl Harness {
    fn load(root: PathBuf, routing_path: Option<PathBuf>) -> Result<Self, String> {
        let routing_file = routing_path.unwrap_or_else(|| root.join("routing.yaml"));
        let mut routing: Routing = read_yaml(&routing_file)?;
        for excluded in &mut routing.excluded {
            excluded.path = expand_config_path(&excluded.path, &root)?;
        }
        for source in &mut routing.knowledge_sources {
            source.path = expand_config_path(&source.path, &root)?;
            source.scopes = source
                .scopes
                .iter()
                .map(|scope| expand_config_path(scope, &root))
                .collect::<Result<Vec<_>, _>>()?;
        }
        let registry_path = root.join("records/tasks/index.yaml");
        let registry = if registry_path.is_file() {
            read_yaml(&registry_path)?
        } else {
            Registry {
                version: 1,
                tasks: vec![],
            }
        };
        if registry.version != 1 {
            return Err(format!(
                "unsupported task registry version: {}",
                registry.version
            ));
        }
        Ok(Self {
            root,
            routing,
            registry,
        })
    }

    fn discover(
        &self,
        task_hint: Option<&str>,
        cwd: &Path,
        allow_planned_hint: bool,
    ) -> Result<&RegistryTask, String> {
        let active: Vec<&RegistryTask> = self
            .registry
            .tasks
            .iter()
            .filter(|task| task.status == "active")
            .collect();
        if let Some(task_id) = task_hint {
            let task = self
                .registry
                .tasks
                .iter()
                .find(|task| {
                    task.task_id == task_id
                        && (task.status == "active"
                            || (allow_planned_hint && task.status == "planned"))
                })
                .ok_or_else(|| format!("task is not active, planned, or registered: {task_id}"))?;
            self.validate_registry_task(task)?;
            return Ok(task);
        }

        let candidates = matching_tasks(&active, cwd);
        match candidates.as_slice() {
            [only] => {
                self.validate_registry_task(only)?;
                Ok(only)
            }
            [] => Err(format!(
                "no unique active task matches cwd {}; specify --task TASK_ID",
                cwd.display()
            )),
            many => Err(format!(
                "multiple active tasks match cwd {}: {}; specify --task TASK_ID",
                cwd.display(),
                many.iter()
                    .map(|task| task.task_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    fn ensure_local_task(&mut self, cwd: &Path) -> Result<(), String> {
        let project = project_root(cwd)?;
        self.reject_non_entry(&project)?;
        let active: Vec<&RegistryTask> = self
            .registry
            .tasks
            .iter()
            .filter(|task| task.status == "active")
            .collect();
        if !matching_tasks(&active, &project).is_empty() {
            return Ok(());
        }

        let task_id = auto_task_id(&project);
        let task_path = format!("records/tasks/{task_id}.yaml");
        let pack_path = format!("records/contexts/{task_id}-context-pack.md");
        let task_file = self.root.join(&task_path);
        let registry_file = self.root.join("records/tasks/index.yaml");
        if task_file.exists() {
            return Err(format!(
                "auto task file already exists: {}",
                task_file.display()
            ));
        }

        let task = AutoTaskFile {
            task_id: task_id.clone(),
            profile: "assessment".to_string(),
            goal: format!(
                "在 {} 繼續目前工作，先確認 scope 與可驗證的下一步。",
                project.display()
            ),
            scope: AutoTaskScope {
                projects: vec![project.display().to_string()],
                services: vec![],
                repositories: vec![project.display().to_string()],
                environments: vec!["local".to_string()],
            },
            excluded_scope: self
                .routing
                .excluded
                .iter()
                .map(|excluded| excluded.path.clone())
                .collect(),
            stop_condition:
                "完成初始 scope 盤點、plan 與下一步決策；需要跨專案或高風險操作時停下。".to_string(),
            current_state: "由 Context Harness 自動建立的本地 task；尚未執行工作。".to_string(),
            context_policy: ContextPolicyOutput {
                layers: TASK_CONTEXT_LAYERS
                    .iter()
                    .map(|layer| (*layer).to_string())
                    .collect(),
                max_items: 10,
                max_chars: 24000,
            },
        };
        let task_yaml = serde_yaml::to_string(&task)
            .map_err(|error| format!("cannot serialize auto task: {error}"))?;
        if let Some(parent) = task_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create auto task directory: {error}"))?;
        }
        fs::write(&task_file, task_yaml)
            .map_err(|error| format!("cannot write auto task: {error}"))?;

        self.registry.tasks.push(RegistryTask {
            task_id,
            status: "active".to_string(),
            task_path,
            pack_path,
            projects: vec![project.display().to_string()],
            preferred_runtimes: vec![],
        });
        let registry_yaml = serde_yaml::to_string(&self.registry)
            .map_err(|error| format!("cannot serialize task registry: {error}"))?;
        if let Some(parent) = registry_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create task registry directory: {error}"))?;
        }
        fs::write(&registry_file, registry_yaml)
            .map_err(|error| format!("cannot write task registry: {error}"))?;
        println!("task_created: local scope {}", project.display());
        Ok(())
    }

    fn validate_registry_task(&self, task: &RegistryTask) -> Result<(), String> {
        validate_task_id(&task.task_id)?;
        let task_path = self.root.join(&task.task_path);
        if !task_path.is_file() {
            return Err(format!("task file does not exist: {}", task_path.display()));
        }
        if task.projects.is_empty() {
            return Err(format!("task has no project scope: {}", task.task_id));
        }
        for project in &task.projects {
            let path = Path::new(project);
            if !path.is_absolute() || !path.exists() {
                return Err(format!("invalid task project scope: {project}"));
            }
            self.reject_non_entry(path)?;
        }
        Ok(())
    }

    fn read_task(&self, registry_task: &RegistryTask) -> Result<Task, String> {
        validate_task_id(&registry_task.task_id)?;
        let task: Task = read_yaml(&self.root.join(&registry_task.task_path))?;
        if task.task_id != registry_task.task_id {
            return Err(format!(
                "registry/task id mismatch: {} != {}",
                registry_task.task_id, task.task_id
            ));
        }
        validate_task_id(&task.task_id)?;
        Ok(task)
    }

    fn state_path(&self, task_id: &str) -> PathBuf {
        self.root
            .join("records/state")
            .join(format!("{task_id}.yaml"))
    }

    fn read_state(&self, task_id: &str) -> Result<Option<TaskState>, String> {
        let path = self.state_path(task_id);
        if !path.is_file() {
            return Ok(None);
        }
        let state: TaskState = read_yaml(&path)?;
        if state.task_id != task_id {
            return Err(format!(
                "state/task id mismatch: {} != {task_id}",
                state.task_id
            ));
        }
        Ok(Some(state))
    }

    fn write_state(&self, state: &TaskState) -> Result<(), String> {
        let path = self.state_path(&state.task_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create state directory: {e}"))?;
        }
        let tmp = path.with_extension(format!("yaml.tmp.{}", std::process::id()));
        let yaml =
            serde_yaml::to_string(state).map_err(|e| format!("cannot serialize state: {e}"))?;
        fs::write(&tmp, yaml).map_err(|e| format!("cannot write temporary state: {e}"))?;
        fs::rename(&tmp, &path).map_err(|e| format!("cannot atomically replace state: {e}"))?;
        Ok(())
    }

    fn make_observation(
        &self,
        task: &Task,
        selected: &RegistryTask,
        runtime: String,
        kind: String,
        scopes: Vec<String>,
        source: String,
        summary: String,
        hypothesis: String,
        evidence: Vec<String>,
    ) -> Result<ObservationEvent, String> {
        const KINDS: [&str; 6] = [
            "correction",
            "explicit_preference",
            "checkpoint",
            "handoff",
            "verification",
            "counterexample",
        ];
        if !KINDS.contains(&kind.as_str()) {
            return Err(format!(
                "unsupported observation kind: {kind}; expected one of {}",
                KINDS.join(", ")
            ));
        }
        validate_observation_enum("runtime", &runtime, &["codex", "claude", "gemini", "grok"])?;
        validate_observation_enum(
            "source",
            &source,
            &[
                "user_message",
                "checkpoint",
                "handoff",
                "verification",
                "runtime_hook",
            ],
        )?;
        validate_observation_text("summary", &summary, 600)?;
        if !hypothesis.is_empty() {
            validate_observation_text("hypothesis", &hypothesis, 600)?;
        }
        if scopes.is_empty() {
            return Err("observe requires at least one scope".to_string());
        }
        let project_roots = selected
            .projects
            .iter()
            .map(|project| canonical_existing_path(Path::new(project), "task project"))
            .collect::<Result<Vec<_>, _>>()?;
        let mut canonical_scopes = Vec::new();
        for scope in &scopes {
            self.reject_non_entry(Path::new(scope))?;
            reject_symlink_components(Path::new(scope))?;
            let path = canonical_existing_path(Path::new(scope), "observation scope")?;
            self.reject_non_entry(&path)?;
            if !project_roots
                .iter()
                .any(|project| path.starts_with(project))
            {
                return Err(format!(
                    "observation scope is outside task projects: {scope}"
                ));
            }
            canonical_scopes.push(path.display().to_string());
        }
        let mut canonical_evidence = Vec::new();
        for path_value in &evidence {
            self.reject_non_entry(Path::new(path_value))?;
            reject_symlink_components(Path::new(path_value))?;
            let path = canonical_existing_path(Path::new(path_value), "observation evidence")?;
            if !path.is_file() {
                return Err(format!(
                    "observation evidence is not a regular file: {path_value}"
                ));
            }
            self.reject_non_entry(&path)?;
            if !project_roots
                .iter()
                .any(|project| path.starts_with(project))
            {
                return Err(format!(
                    "observation evidence is outside task projects: {path_value}"
                ));
            }
            canonical_evidence.push(path.display().to_string());
        }
        Ok(ObservationEvent {
            version: 1,
            observation_id: format!("obs-{}-{}", unix_nanos(), std::process::id()),
            task_id: task.task_id.clone(),
            created_at: unix_timestamp(),
            runtime,
            kind,
            scope: canonical_scopes,
            source,
            summary,
            hypothesis,
            evidence: canonical_evidence,
            status: "candidate".to_string(),
            confidence: "low".to_string(),
        })
    }

    fn write_observation(&self, event: &ObservationEvent) -> Result<PathBuf, String> {
        let directory = self.observation_directory()?;
        reject_symlink_components(&directory)?;
        fs::create_dir_all(&directory)
            .map_err(|e| format!("cannot create observation directory: {e}"))?;
        reject_symlink_components(&directory)?;
        if let Some(base) = directory.parent() {
            #[cfg(unix)]
            fs::set_permissions(base, fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("cannot set observation base permissions: {e}"))?;
        }
        #[cfg(unix)]
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot set observation directory permissions: {e}"))?;
        let path = directory.join(format!("{}.yaml", event.observation_id));
        let yaml = serde_yaml::to_string(event)
            .map_err(|e| format!("cannot serialize observation: {e}"))?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(&path)
            .map_err(|e| format!("cannot create observation without overwrite: {e}"))?;
        file.write_all(yaml.as_bytes())
            .map_err(|e| format!("cannot write observation: {e}"))?;
        #[cfg(unix)]
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("cannot set observation permissions: {e}"))?;
        Ok(path)
    }

    fn organize_observations(
        &self,
        task: &Task,
        selected: &RegistryTask,
    ) -> Result<OrganizationReport, String> {
        self.recover_feedback_journals()?;
        let observation_directory = self.observation_directory()?;
        let observation_metadata = match fs::symlink_metadata(&observation_directory) {
            Ok(value) => value,
            Err(_) => {
                return Ok(OrganizationReport {
                    candidate_count: 0,
                    observation_count: 0,
                    rejected_count: 0,
                    paths: vec![],
                })
            }
        };
        if !observation_metadata.file_type().is_dir() {
            return Err("observation directory must be a real directory".to_string());
        }
        if let Some(base) = observation_directory.parent() {
            validate_private_directory(base)?;
        }
        validate_private_directory(&observation_directory)?;
        if !observation_directory.is_dir() {
            return Ok(OrganizationReport {
                candidate_count: 0,
                observation_count: 0,
                rejected_count: 0,
                paths: vec![],
            });
        }

        reject_symlink_components(&observation_directory)?;
        let mut groups: BTreeMap<(String, Vec<String>), Vec<ObservationEvent>> = BTreeMap::new();
        let mut observation_count = 0usize;
        let mut rejected_count = 0usize;
        for entry in fs::read_dir(&observation_directory)
            .map_err(|e| format!("cannot read observation directory: {e}"))?
        {
            let path = entry
                .map_err(|e| format!("cannot read observation directory entry: {e}"))?
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
                continue;
            }
            if fs::metadata(&path)
                .map_err(|e| format!("cannot stat observation: {e}"))?
                .len()
                > 64 * 1024
            {
                rejected_count += 1;
                continue;
            }
            let metadata = match fs::symlink_metadata(&path) {
                Ok(value) => value,
                Err(_) => {
                    rejected_count += 1;
                    continue;
                }
            };
            if !metadata.file_type().is_file() {
                rejected_count += 1;
                continue;
            }
            let event: ObservationEvent = match read_yaml(&path) {
                Ok(value) => value,
                Err(_) => {
                    rejected_count += 1;
                    continue;
                }
            };
            if event.task_id != task.task_id || event.status != "candidate" {
                continue;
            }
            observation_count += 1;
            if self
                .validate_observation_for_organization(&event, selected)
                .is_err()
            {
                rejected_count += 1;
                continue;
            }
            let key = candidate_group_key(&event);
            groups.entry(key).or_default().push(event);
        }

        let candidate_directory = self.candidate_directory()?;
        reject_symlink_components(&candidate_directory)?;
        fs::create_dir_all(&candidate_directory)
            .map_err(|e| format!("cannot create candidate directory: {e}"))?;
        reject_symlink_components(&candidate_directory)?;
        if let Some(base) = candidate_directory.parent() {
            validate_private_directory(base)?;
        }
        #[cfg(unix)]
        fs::set_permissions(&candidate_directory, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot set candidate directory permissions: {e}"))?;
        let mut paths = Vec::new();
        for (key, events) in groups {
            let mut events = events;
            events.sort_by(|left, right| {
                left.kind
                    .cmp(&right.kind)
                    .then(left.observation_id.cmp(&right.observation_id))
            });
            let first = &events[0];
            let mut kinds = events
                .iter()
                .map(|event| event.kind.clone())
                .collect::<Vec<_>>();
            kinds.sort();
            kinds.dedup();
            let mut observation_ids = events
                .iter()
                .map(|event| event.observation_id.clone())
                .collect::<Vec<_>>();
            observation_ids.sort();
            let mut counterexample_observation_ids = events
                .iter()
                .filter(|event| event.kind == "counterexample")
                .map(|event| event.observation_id.clone())
                .collect::<Vec<_>>();
            counterexample_observation_ids.sort();
            let mut sources = events
                .iter()
                .map(|event| format!("{} via {}", event.source, event.runtime))
                .collect::<Vec<_>>();
            sources.sort();
            sources.dedup();
            let candidate_material = format!("{}|{}", task.task_id, candidate_key_material(&key));
            let mut candidate = CandidateRecord {
                version: 1,
                candidate_id: format!(
                    "candidate-{:016x}",
                    stable_hash(candidate_material.as_bytes())
                ),
                status: "candidate".to_string(),
                confidence: "low".to_string(),
                task_id: task.task_id.clone(),
                observation_ids,
                kinds,
                counterexample_observation_ids,
                sources,
                source: format!("{} via {}", first.source, first.runtime),
                observation: first.summary.clone(),
                hypothesis: first.hypothesis.clone(),
                scope: first.scope.clone(),
                promotion_condition: "Jev 可判斷是否納入可攜經驗庫；候選不因此改變正式規則。"
                    .to_string(),
                next_action: "sync 時由 Jev 判斷保留、略過或交由人工確認；含反例或疑似敏感內容時保留人工閘門。"
                    .to_string(),
                reviewed_at: String::new(),
                review_reason: String::new(),
                jev_review_observation_count: 0,
                decision_history: Vec::new(),
                owner: String::new(),
                feedback_history: Vec::new(),
                identity: candidate_material,
            };
            let path = candidate_directory.join(format!("{}.yaml", candidate.candidate_id));
            let _lock = acquire_candidate_lock(&candidate_directory, &candidate.candidate_id)?;
            let existing_metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => Some(metadata),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(format!("cannot inspect candidate destination: {error}"));
                }
            };
            if let Some(metadata) = existing_metadata {
                if !metadata.file_type().is_file() {
                    if metadata.file_type().is_symlink() {
                        fs::remove_file(&path).map_err(|e| {
                            format!("cannot remove candidate symlink without following it: {e}")
                        })?;
                    } else {
                        return Err(format!(
                            "candidate destination must be a regular file: {}",
                            path.display()
                        ));
                    }
                } else {
                    validate_private_file(&path)?;
                    let existing: CandidateRecord = read_yaml(&path)?;
                    let expected_id = path
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .ok_or("candidate filename is invalid".to_string())?;
                    if existing.candidate_id != expected_id {
                        return Err("candidate id does not match its filename".to_string());
                    }
                    if !existing.identity.is_empty() && existing.identity != candidate.identity {
                        return Err(
                            "candidate id collision detected for different identity".to_string()
                        );
                    }
                    let only_jev_reviews = has_only_jev_reviews(&existing.decision_history);
                    if existing.status != "candidate"
                        || (!existing.decision_history.is_empty() && !only_jev_reviews)
                        || !existing.feedback_history.is_empty()
                        || !existing.owner.is_empty()
                    {
                        paths.push(path);
                        continue;
                    }
                    if only_jev_reviews {
                        candidate.reviewed_at = existing.reviewed_at;
                        candidate.review_reason = existing.review_reason;
                        candidate.jev_review_observation_count =
                            if existing.jev_review_observation_count == 0 {
                                // Older candidate records did not store the evidence count.
                                existing.observation_ids.len()
                            } else {
                                existing.jev_review_observation_count
                            };
                        candidate.decision_history = existing.decision_history;
                    }
                }
            }
            let yaml = serde_yaml::to_string(&candidate)
                .map_err(|e| format!("cannot serialize candidate: {e}"))?;
            let temp_path = candidate_directory.join(format!(
                ".{}.tmp-{}",
                candidate.candidate_id,
                unix_nanos()
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options
                .open(&temp_path)
                .map_err(|e| format!("cannot create candidate safely: {e}"))?;
            file.write_all(yaml.as_bytes())
                .map_err(|e| format!("cannot write candidate safely: {e}"))?;
            #[cfg(unix)]
            fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("cannot set candidate permissions: {e}"))?;
            fs::rename(&temp_path, &path)
                .map_err(|e| format!("cannot publish candidate safely: {e}"))?;
            paths.push(path);
        }
        Ok(OrganizationReport {
            candidate_count: paths.len(),
            observation_count,
            rejected_count,
            paths,
        })
    }

    fn validate_observation_for_organization(
        &self,
        event: &ObservationEvent,
        selected: &RegistryTask,
    ) -> Result<(), String> {
        if event.version != 1 {
            return Err(format!(
                "unsupported observation version: {}",
                event.version
            ));
        }
        const KINDS: [&str; 6] = [
            "correction",
            "explicit_preference",
            "checkpoint",
            "handoff",
            "verification",
            "counterexample",
        ];
        if !KINDS.contains(&event.kind.as_str()) {
            return Err(format!("unsupported observation kind: {}", event.kind));
        }
        if event.status != "candidate" || event.confidence != "low" {
            return Err("organization only accepts candidate/low observations".to_string());
        }
        if !valid_observation_id(&event.observation_id) || !valid_timestamp(&event.created_at) {
            return Err("observation provenance has invalid id or timestamp".to_string());
        }
        validate_observation_enum(
            "runtime",
            &event.runtime,
            &["codex", "claude", "gemini", "grok"],
        )?;
        validate_observation_enum(
            "source",
            &event.source,
            &[
                "user_message",
                "checkpoint",
                "handoff",
                "verification",
                "runtime_hook",
            ],
        )?;
        validate_observation_text("summary", &event.summary, 600)?;
        if !event.hypothesis.is_empty() {
            validate_observation_text("hypothesis", &event.hypothesis, 600)?;
        }
        if event.scope.is_empty() {
            return Err("candidate observation requires at least one scope".to_string());
        }
        if event.scope.len() > 16 || event.evidence.len() > 16 {
            return Err("candidate observation has too many scope or evidence paths".to_string());
        }
        let roots = selected
            .projects
            .iter()
            .map(|project| canonical_existing_path(Path::new(project), "task project"))
            .collect::<Result<Vec<_>, _>>()?;
        for scope in &event.scope {
            self.reject_non_entry(Path::new(scope))?;
            reject_symlink_components(Path::new(scope))?;
            let path = canonical_existing_path(Path::new(scope), "candidate scope")?;
            if !roots.iter().any(|root| path.starts_with(root)) {
                return Err(format!("candidate scope is outside task projects: {scope}"));
            }
        }
        for evidence in &event.evidence {
            self.reject_non_entry(Path::new(evidence))?;
            reject_symlink_components(Path::new(evidence))?;
            let path = canonical_existing_path(Path::new(evidence), "candidate evidence")?;
            if !path.is_file() {
                return Err(format!(
                    "candidate evidence is not a regular file: {evidence}"
                ));
            }
            if !roots.iter().any(|root| path.starts_with(root)) {
                return Err(format!(
                    "candidate evidence is outside task projects: {evidence}"
                ));
            }
        }
        Ok(())
    }

    fn candidate_directory(&self) -> Result<PathBuf, String> {
        let observations = self.observation_directory()?;
        observations
            .parent()
            .map(|parent| parent.join("candidates"))
            .ok_or("cannot determine private candidate directory".to_string())
    }

    fn export_experience_pack(&self, output: &Path) -> Result<(PathBuf, usize), String> {
        if !output.is_absolute() {
            return Err("export output must be an absolute path".to_string());
        }
        self.reject_non_entry(output)?;
        reject_symlink_components(output)?;
        if output.exists() {
            return Err(format!(
                "export output already exists: {}",
                output.display()
            ));
        }
        let directory = self.candidate_directory()?;
        validate_private_directory(&directory)?;
        let mut candidates = Vec::new();
        let mut source_roots = Vec::new();
        for entry in
            fs::read_dir(&directory).map_err(|e| format!("cannot read candidate directory: {e}"))?
        {
            let path = entry
                .map_err(|e| format!("cannot read candidate entry: {e}"))?
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
                continue;
            }
            validate_private_file(&path)?;
            let candidate: CandidateRecord = read_yaml(&path)?;
            if candidate.status != "confirmed" {
                continue;
            }
            validate_candidate_id(&candidate.candidate_id)?;
            validate_owner(&candidate.owner)?;
            for scope in &candidate.scope {
                self.reject_non_entry(Path::new(scope))?;
                let canonical = canonical_existing_path(Path::new(scope), "experience scope")?;
                source_roots.push(canonical.display().to_string());
            }
            candidates.push(candidate);
        }
        source_roots.sort();
        source_roots.dedup();
        candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
        let pack = PortableExperiencePack {
            version: 1,
            kind: "agent-experience-pack".to_string(),
            created_at: unix_timestamp(),
            source: "confirmed candidate store; summaries only".to_string(),
            source_roots,
            candidates,
        };
        let yaml = serde_yaml::to_string(&pack)
            .map_err(|e| format!("cannot serialize experience pack: {e}"))?;
        let parent = output
            .parent()
            .ok_or("export output has no parent directory".to_string())?;
        if !parent.is_dir() {
            return Err(format!(
                "export parent directory does not exist: {}",
                parent.display()
            ));
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(output)
            .map_err(|e| format!("cannot create export pack: {e}"))?;
        file.write_all(yaml.as_bytes())
            .map_err(|e| format!("cannot write export pack: {e}"))?;
        Ok((output.to_path_buf(), pack.candidates.len()))
    }

    fn import_experience_pack(
        &self,
        input: &Path,
        mappings: &[(PathBuf, PathBuf)],
    ) -> Result<(usize, usize), String> {
        if !input.is_absolute() {
            return Err("import input must be an absolute path".to_string());
        }
        self.reject_non_entry(input)?;
        let metadata =
            fs::symlink_metadata(input).map_err(|e| format!("cannot inspect import pack: {e}"))?;
        if !metadata.file_type().is_file() {
            return Err("import pack must be a regular non-symlink file".to_string());
        }
        if fs::metadata(input)
            .map_err(|e| format!("cannot stat import pack: {e}"))?
            .len()
            > 8 * 1024 * 1024
        {
            return Err("import pack is too large".to_string());
        }
        let pack: PortableExperiencePack = read_yaml(input)?;
        if pack.version != 1
            || !matches!(
                pack.kind.as_str(),
                "agent-experience-pack" | "miyago-agent-experience-pack"
            )
        {
            return Err("unsupported experience pack".to_string());
        }
        let directory = self.candidate_directory()?;
        reject_symlink_components(&directory)?;
        fs::create_dir_all(&directory)
            .map_err(|e| format!("cannot create candidate directory: {e}"))?;
        #[cfg(unix)]
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot set candidate directory permissions: {e}"))?;
        let mut imported = 0usize;
        let mut skipped = 0usize;
        for mut candidate in pack.candidates {
            if candidate.status != "confirmed" {
                return Err("experience pack may contain confirmed candidates only".to_string());
            }
            validate_candidate_id(&candidate.candidate_id)?;
            validate_owner(&candidate.owner)?;
            candidate.scope = candidate
                .scope
                .iter()
                .map(|scope| remap_import_scope(scope, mappings))
                .collect::<Result<Vec<_>, _>>()?;
            for scope in &candidate.scope {
                self.reject_non_entry(Path::new(scope))?;
                let canonical = canonical_existing_path(Path::new(scope), "import scope")?;
                if !canonical.is_dir() {
                    return Err(format!(
                        "import scope is not a directory: {}",
                        canonical.display()
                    ));
                }
            }
            let (normalized, mut scope) =
                candidate_group_material(&candidate.observation, &candidate.scope);
            let identity = format!(
                "{}|{}",
                candidate.task_id,
                candidate_key_material(&(normalized, scope.clone()))
            );
            let candidate_id = format!("candidate-{:016x}", stable_hash(identity.as_bytes()));
            candidate.candidate_id = candidate_id.clone();
            candidate.identity = identity;
            candidate.sources.push("portable_import".to_string());
            candidate.sources.sort();
            candidate.sources.dedup();
            scope.sort();
            scope.dedup();
            candidate.scope = scope;
            let path = directory.join(format!("{candidate_id}.yaml"));
            if path.exists() {
                validate_private_file(&path)?;
                let existing: CandidateRecord = read_yaml(&path)?;
                if existing.identity == candidate.identity {
                    skipped += 1;
                    continue;
                }
                return Err(format!("import candidate collision: {candidate_id}"));
            }
            let yaml = serde_yaml::to_string(&candidate)
                .map_err(|e| format!("cannot serialize imported candidate: {e}"))?;
            self.publish_private_new_yaml(&directory, &path, &yaml)?;
            imported += 1;
        }
        Ok((imported, skipped))
    }

    fn build_review_queue(&self, task: &Task) -> Result<PathBuf, String> {
        let candidate_directory = self.candidate_directory()?;
        let review_directory = candidate_directory
            .parent()
            .ok_or("cannot determine private review directory".to_string())?
            .join("reviews");
        reject_symlink_components(&review_directory)?;
        fs::create_dir_all(&review_directory)
            .map_err(|e| format!("cannot create review directory: {e}"))?;
        reject_symlink_components(&review_directory)?;
        validate_private_directory(
            review_directory
                .parent()
                .ok_or("cannot determine private review parent".to_string())?,
        )?;
        #[cfg(unix)]
        fs::set_permissions(&review_directory, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot set review directory permissions: {e}"))?;

        let mut candidates = Vec::new();
        if let Ok(metadata) = fs::symlink_metadata(&candidate_directory) {
            if !metadata.file_type().is_dir() {
                return Err("candidate directory must be a real directory".to_string());
            }
            validate_private_directory(&candidate_directory)?;
            for entry in fs::read_dir(&candidate_directory)
                .map_err(|e| format!("cannot read candidate directory: {e}"))?
            {
                let path = entry
                    .map_err(|e| format!("cannot read candidate entry: {e}"))?
                    .path();
                if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
                    continue;
                }
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|e| format!("cannot inspect candidate entry: {e}"))?;
                if !metadata.file_type().is_file() {
                    continue;
                }
                validate_private_file(&path)?;
                let candidate: CandidateRecord = read_yaml(&path)?;
                let expected_id = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .ok_or("candidate filename is invalid".to_string())?;
                validate_candidate_id(expected_id)?;
                if candidate.candidate_id != expected_id {
                    return Err("candidate id does not match its filename".to_string());
                }
                if candidate.task_id == task.task_id && candidate.status == "candidate" {
                    let jev_review_reason = candidate
                        .decision_history
                        .iter()
                        .rev()
                        .find(|entry| entry.decision == "jev_review")
                        .map(|entry| entry.reason.as_str());
                    let action = if !candidate.counterexample_observation_ids.is_empty() {
                        "候選包含反例，需人工確認；不自動納入可攜經驗庫".to_string()
                    } else {
                        match jev_review_reason {
                            Some(reason) => {
                                format!("Jev 判斷需要人工確認；尚未加入可攜經驗庫（{reason}）")
                            }
                            None => "需要使用者明確確認；不自動寫回正式規則".to_string(),
                        }
                    };
                    candidates.push(ReviewItem {
                        candidate_id: candidate.candidate_id,
                        observation_count: candidate.observation_ids.len(),
                        source_count: candidate.sources.len(),
                        kinds: candidate.kinds,
                        counterexample_count: candidate.counterexample_observation_ids.len(),
                        action,
                    });
                }
            }
        }
        candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
        let queue = ReviewQueue {
            version: 1,
            task_id: task.task_id.clone(),
            generated_at: unix_timestamp(),
            human_gate: "required".to_string(),
            candidates,
        };
        let path = review_directory.join(format!("{}-queue.yaml", task.task_id));
        let yaml = serde_yaml::to_string(&queue)
            .map_err(|e| format!("cannot serialize review queue: {e}"))?;
        self.publish_private_yaml(&review_directory, &path, &yaml)?;
        Ok(path)
    }

    fn decide_candidate(
        &self,
        task: &Task,
        candidate_id: &str,
        decision: &str,
        reason: &str,
        owner: &str,
    ) -> Result<PathBuf, String> {
        self.recover_feedback_journals()?;
        validate_candidate_id(candidate_id)?;
        let status = match decision {
            "confirm" => "confirmed",
            "reject" => "rejected",
            "revoke" => "superseded",
            "restore" => "confirmed",
            _ => return Err("decision must be confirm, reject, revoke, or restore".to_string()),
        };
        if decision != "confirm" && reason.trim().is_empty() {
            return Err("reject/revoke/restore requires --reason".to_string());
        }
        if decision == "confirm" || decision == "restore" {
            validate_owner(owner)?;
        }
        let directory = self.candidate_directory()?;
        validate_private_directory(&directory)?;
        let path = directory.join(format!("{candidate_id}.yaml"));
        let _lock = acquire_candidate_lock(&directory, candidate_id)?;
        let metadata =
            fs::symlink_metadata(&path).map_err(|e| format!("cannot inspect candidate: {e}"))?;
        if !metadata.file_type().is_file() {
            return Err("candidate must be a regular file".to_string());
        }
        validate_private_file(&path)?;
        let mut candidate: CandidateRecord = read_yaml(&path)?;
        if candidate.candidate_id != candidate_id {
            return Err("candidate id does not match its filename".to_string());
        }
        let valid_transition = match decision {
            "confirm" => candidate.status == "candidate",
            "reject" => candidate.status == "candidate",
            "revoke" => candidate.status == "confirmed",
            "restore" => candidate.status == "retired",
            _ => false,
        };
        if candidate.task_id != task.task_id || !valid_transition {
            return Err("candidate is not pending for this task".to_string());
        }
        candidate.status = status.to_string();
        if decision == "confirm" || decision == "restore" {
            candidate.owner = owner.to_string();
        }
        candidate.reviewed_at = unix_timestamp();
        candidate.review_reason = reason.to_string();
        candidate.decision_history.push(DecisionRecord {
            decision: decision.to_string(),
            at: candidate.reviewed_at.clone(),
            reason: reason.to_string(),
        });
        let yaml = serde_yaml::to_string(&candidate)
            .map_err(|e| format!("cannot serialize candidate decision: {e}"))?;
        self.invalidate_consumption_bundles(candidate_id)?;
        self.publish_private_yaml(&directory, &path, &yaml)?;
        Ok(path)
    }

    fn record_jev_review(
        &self,
        task: &Task,
        candidate_id: &str,
        reason: &str,
    ) -> Result<PathBuf, String> {
        self.recover_feedback_journals()?;
        validate_candidate_id(candidate_id)?;
        let directory = self.candidate_directory()?;
        validate_private_directory(&directory)?;
        let path = directory.join(format!("{candidate_id}.yaml"));
        let _lock = acquire_candidate_lock(&directory, candidate_id)?;
        let metadata =
            fs::symlink_metadata(&path).map_err(|e| format!("cannot inspect candidate: {e}"))?;
        if !metadata.file_type().is_file() {
            return Err("candidate must be a regular file".to_string());
        }
        validate_private_file(&path)?;
        let mut candidate: CandidateRecord = read_yaml(&path)?;
        if candidate.candidate_id != candidate_id
            || candidate.task_id != task.task_id
            || candidate.status != "candidate"
        {
            return Err("candidate is not pending Jev review for this task".to_string());
        }
        candidate.reviewed_at = unix_timestamp();
        candidate.review_reason = reason.to_string();
        candidate.jev_review_observation_count = candidate.observation_ids.len();
        candidate.decision_history.push(DecisionRecord {
            decision: "jev_review".to_string(),
            at: candidate.reviewed_at.clone(),
            reason: reason.to_string(),
        });
        let yaml = serde_yaml::to_string(&candidate)
            .map_err(|e| format!("cannot serialize Jev review: {e}"))?;
        self.publish_private_yaml(&directory, &path, &yaml)?;
        Ok(path)
    }

    fn resolve_decision_candidate(
        &self,
        task: &Task,
        requested_id: Option<&str>,
        decision: &str,
    ) -> Result<String, String> {
        if let Some(candidate_id) = requested_id {
            validate_candidate_id(candidate_id)?;
            return Ok(candidate_id.to_string());
        }

        let expected_status = match decision {
            "confirm" | "reject" => "candidate",
            "revoke" => "confirmed",
            "restore" => "retired",
            _ => return Err("decision must be confirm, reject, revoke, or restore".to_string()),
        };
        let directory = self.candidate_directory()?;
        validate_private_directory(&directory)?;
        let mut matches = Vec::new();
        for entry in
            fs::read_dir(&directory).map_err(|e| format!("cannot read candidate directory: {e}"))?
        {
            let path = entry
                .map_err(|e| format!("cannot read candidate entry: {e}"))?
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
                continue;
            }
            validate_private_file(&path)?;
            let candidate: CandidateRecord = read_yaml(&path)?;
            if candidate.task_id == task.task_id && candidate.status == expected_status {
                matches.push(candidate.candidate_id);
            }
        }
        matches.sort();
        match matches.as_slice() {
            [candidate_id] => Ok(candidate_id.clone()),
            [] => Err(format!(
                "no {expected_status} candidate is eligible for {decision}; pass --candidate ID"
            )),
            _ => Err(format!(
                "multiple {expected_status} candidates found ({}); pass --candidate ID",
                matches.join(", ")
            )),
        }
    }

    fn consume_confirmed(
        &self,
        task: &Task,
        runtime: &str,
        candidate_id: Option<&str>,
        scope_filters: Option<&[String]>,
    ) -> Result<PathBuf, String> {
        self.recover_feedback_journals()?;
        validate_observation_enum("runtime", runtime, &["codex", "claude", "gemini", "grok"])?;
        if let Some(candidate_id) = candidate_id {
            validate_candidate_id(candidate_id)?;
        }
        let directory = self.candidate_directory()?;
        validate_private_directory(&directory)?;
        let mut candidates = Vec::new();
        let mut locks = Vec::new();
        for entry in
            fs::read_dir(&directory).map_err(|e| format!("cannot read candidate directory: {e}"))?
        {
            let path = entry
                .map_err(|e| format!("cannot read candidate entry: {e}"))?
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
                continue;
            }
            validate_private_file(&path)?;
            let expected_id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or("candidate filename is invalid".to_string())?;
            validate_candidate_id(expected_id)?;
            locks.push(acquire_candidate_lock(&directory, expected_id)?);
            let candidate: CandidateRecord = read_yaml(&path)?;
            if candidate.candidate_id != expected_id {
                return Err("candidate id does not match its filename".to_string());
            }
            if candidate.task_id != task.task_id || candidate.status != "confirmed" {
                continue;
            }
            if candidate_id.is_some_and(|value| value != candidate.candidate_id) {
                continue;
            }
            if scope_filters.is_some_and(|filters| {
                !filters.iter().any(|filter| {
                    candidate
                        .scope
                        .iter()
                        .any(|scope| scopes_overlap(filter, scope))
                })
            }) {
                continue;
            }
            validate_owner(&candidate.owner)?;
            candidates.push(ConsumedExperience {
                candidate_id: candidate.candidate_id,
                owner: candidate.owner,
                observation: candidate.observation,
                hypothesis: candidate.hypothesis,
                scope: candidate.scope,
                sources: candidate.sources,
            });
        }
        if let Some(candidate_id) = candidate_id {
            if !candidates
                .iter()
                .any(|value| value.candidate_id == candidate_id)
            {
                return Err("confirmed candidate was not found for this task".to_string());
            }
        }
        candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
        let bundle = ExperienceBundle {
            version: 1,
            task_id: task.task_id.clone(),
            runtime: runtime.to_string(),
            generated_at: unix_timestamp(),
            source: "confirmed experience candidate store; no raw transcript".to_string(),
            experiences: candidates,
        };
        let base = directory
            .parent()
            .ok_or("cannot determine private consumption directory".to_string())?;
        let output_directory = base.join("consumptions");
        reject_symlink_components(&output_directory)?;
        fs::create_dir_all(&output_directory)
            .map_err(|e| format!("cannot create consumption directory: {e}"))?;
        #[cfg(unix)]
        fs::set_permissions(&output_directory, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot set consumption directory permissions: {e}"))?;
        let path = output_directory.join(format!("{}-{runtime}.yaml", task.task_id));
        let yaml = serde_yaml::to_string(&bundle)
            .map_err(|e| format!("cannot serialize experience bundle: {e}"))?;
        self.publish_private_yaml(&output_directory, &path, &yaml)?;
        Ok(path)
    }

    fn invalidate_consumption_bundles(&self, candidate_id: &str) -> Result<(), String> {
        validate_candidate_id(candidate_id)?;
        let candidate_directory = self.candidate_directory()?;
        let directory = candidate_directory
            .parent()
            .ok_or("cannot determine private consumption directory".to_string())?
            .join("consumptions");
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("cannot inspect consumption directory: {error}")),
        };
        if !metadata.file_type().is_dir() {
            return Err("consumption sink must be a real directory".to_string());
        }
        validate_private_directory(&directory)?;
        for entry in fs::read_dir(&directory)
            .map_err(|e| format!("cannot read consumption directory: {e}"))?
        {
            let path = entry
                .map_err(|e| format!("cannot read consumption entry: {e}"))?
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
                continue;
            }
            validate_private_file(&path)?;
            let bundle: ExperienceBundle = read_yaml(&path)?;
            if bundle
                .experiences
                .iter()
                .any(|experience| experience.candidate_id == candidate_id)
            {
                fs::remove_file(&path)
                    .map_err(|e| format!("cannot invalidate consumption bundle: {e}"))?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn record_feedback(
        &self,
        task: &Task,
        selected: &RegistryTask,
        candidate_id: &str,
        runtime: &str,
        kind: &str,
        action: &str,
        summary: &str,
        scopes: Vec<String>,
        source: &str,
    ) -> Result<(PathBuf, String), String> {
        self.recover_feedback_journals()?;
        validate_candidate_id(candidate_id)?;
        validate_observation_enum("runtime", runtime, &["codex", "claude", "gemini", "grok"])?;
        validate_observation_enum("kind", kind, &["correction", "counterexample"])?;
        validate_observation_enum(
            "source",
            source,
            &[
                "user_message",
                "checkpoint",
                "handoff",
                "verification",
                "runtime_hook",
            ],
        )?;
        validate_observation_text("summary", summary, 600)?;
        if !["narrow", "downgrade", "retire"].contains(&action) {
            return Err("feedback action must be narrow, downgrade, or retire".to_string());
        }
        if scopes.is_empty() {
            return Err("feedback requires at least one explicit --scope".to_string());
        }
        let directory = self.candidate_directory()?;
        validate_private_directory(&directory)?;
        let _lock = acquire_candidate_lock(&directory, candidate_id)?;
        let path = directory.join(format!("{candidate_id}.yaml"));
        validate_private_file(&path)?;
        let mut candidate: CandidateRecord = read_yaml(&path)?;
        if candidate.candidate_id != candidate_id {
            return Err("candidate id does not match its filename".to_string());
        }
        if candidate.task_id != task.task_id || candidate.status != "confirmed" {
            return Err("feedback requires a confirmed candidate for this task".to_string());
        }
        let event = self.make_observation(
            task,
            selected,
            runtime.to_string(),
            kind.to_string(),
            scopes,
            source.to_string(),
            summary.to_string(),
            String::new(),
            Vec::new(),
        )?;
        if action == "narrow" && !is_strictly_narrower(&event.scope, &candidate.scope) {
            return Err("narrow feedback must strictly reduce the candidate scope".to_string());
        }
        let original_candidate_yaml = fs::read_to_string(&path)
            .map_err(|e| format!("cannot snapshot candidate before feedback: {e}"))?;
        let previous_status = candidate.status.clone();
        let resulting_status = match action {
            "narrow" => {
                candidate.scope = event.scope.clone();
                "confirmed"
            }
            "downgrade" => {
                candidate.confidence = "low".to_string();
                "candidate"
            }
            "retire" => "retired",
            _ => unreachable!(),
        };
        candidate.status = resulting_status.to_string();
        candidate.reviewed_at = unix_timestamp();
        candidate.review_reason = format!("feedback: {summary}");
        let feedback_id = format!("feedback-{}-{}", unix_nanos(), std::process::id());
        candidate.feedback_history.push(FeedbackReference {
            feedback_id: feedback_id.clone(),
            observation_id: event.observation_id.clone(),
            action: action.to_string(),
            at: candidate.reviewed_at.clone(),
            summary: summary.to_string(),
        });
        let candidate_yaml = serde_yaml::to_string(&candidate)
            .map_err(|e| format!("cannot serialize feedback candidate: {e}"))?;
        let feedback_directory = directory
            .parent()
            .ok_or("cannot determine private feedback directory".to_string())?
            .join("feedback");
        reject_symlink_components(&feedback_directory)?;
        fs::create_dir_all(&feedback_directory)
            .map_err(|e| format!("cannot create feedback directory: {e}"))?;
        #[cfg(unix)]
        fs::set_permissions(&feedback_directory, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot set feedback directory permissions: {e}"))?;
        let feedback = FeedbackRecord {
            version: 1,
            feedback_id: feedback_id.clone(),
            candidate_id: candidate_id.to_string(),
            task_id: task.task_id.clone(),
            runtime: runtime.to_string(),
            kind: kind.to_string(),
            action: action.to_string(),
            summary: summary.to_string(),
            observation_id: event.observation_id.clone(),
            previous_status,
            resulting_status: resulting_status.to_string(),
            created_at: unix_timestamp(),
        };
        let feedback_path = feedback_directory.join(format!("{feedback_id}.yaml"));
        let feedback_yaml = serde_yaml::to_string(&feedback)
            .map_err(|e| format!("cannot serialize feedback record: {e}"))?;
        let observation_path = self
            .observation_directory()?
            .join(format!("{}.yaml", event.observation_id));
        let journal_path = feedback_directory.join(format!(".{feedback_id}.pending.yaml"));
        let journal = FeedbackJournal {
            version: 1,
            feedback_id: feedback_id.clone(),
            candidate_path: path.display().to_string(),
            original_candidate_yaml: original_candidate_yaml.clone(),
            observation_path: observation_path.display().to_string(),
            feedback_path: feedback_path.display().to_string(),
            owner_pid: std::process::id(),
            created_at: unix_timestamp(),
            committed: false,
        };
        let journal_yaml = serde_yaml::to_string(&journal)
            .map_err(|e| format!("cannot serialize feedback journal: {e}"))?;
        self.publish_private_new_yaml(&feedback_directory, &journal_path, &journal_yaml)?;
        if let Err(error) = self.write_observation(&event) {
            let _ = fs::remove_file(&journal_path);
            return Err(format!("feedback observation publish failed: {error}"));
        }
        if let Err(error) = self.publish_private_yaml(&directory, &path, &candidate_yaml) {
            let _ = fs::remove_file(&observation_path);
            let _ = fs::remove_file(&journal_path);
            return Err(format!("feedback candidate publish failed: {error}"));
        }
        if let Err(error) =
            self.publish_private_new_yaml(&feedback_directory, &feedback_path, &feedback_yaml)
        {
            let restore_result =
                self.publish_private_yaml(&directory, &path, &original_candidate_yaml);
            let _ = fs::remove_file(&observation_path);
            return match restore_result {
                Ok(()) => {
                    let _ = fs::remove_file(&journal_path);
                    Err(format!("feedback record publish failed; candidate restored: {error}"))
                }
                Err(restore_error) => Err(format!(
                    "feedback record publish failed and candidate restore failed: {error}; {restore_error}"
                )),
            };
        }
        self.invalidate_consumption_bundles(candidate_id)?;
        let committed_journal = FeedbackJournal {
            committed: true,
            ..journal
        };
        let committed_yaml = serde_yaml::to_string(&committed_journal)
            .map_err(|e| format!("cannot serialize committed feedback journal: {e}"))?;
        self.publish_private_yaml(&feedback_directory, &journal_path, &committed_yaml)?;
        match fs::remove_file(&journal_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("cannot remove committed feedback journal: {error}")),
        }
        Ok((feedback_path, resulting_status.to_string()))
    }

    fn recover_feedback_journals(&self) -> Result<(), String> {
        let base = self
            .observation_directory()?
            .parent()
            .ok_or("cannot determine private feedback directory".to_string())?
            .to_path_buf();
        let directory = base.join("feedback");
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("cannot inspect feedback directory: {error}")),
        };
        if !metadata.file_type().is_dir() {
            return Err("feedback sink must be a real directory".to_string());
        }
        validate_private_directory(&directory)?;
        for entry in
            fs::read_dir(&directory).map_err(|e| format!("cannot read feedback directory: {e}"))?
        {
            let path = entry
                .map_err(|e| format!("cannot read feedback entry: {e}"))?
                .path();
            if !path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.ends_with(".pending.yaml"))
            {
                continue;
            }
            validate_private_file(&path)?;
            let journal: FeedbackJournal = read_yaml(&path)?;
            if journal.committed {
                match fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(format!("cannot remove committed feedback journal: {error}"))
                    }
                }
                continue;
            }
            let candidate_path = PathBuf::from(&journal.candidate_path);
            let candidate_directory = self.candidate_directory()?;
            let candidate_id = candidate_path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or("feedback journal candidate filename is invalid".to_string())?;
            validate_candidate_id(candidate_id)?;
            validate_private_target(&candidate_path, &candidate_directory, candidate_id, true)?;
            let observation_directory = self.observation_directory()?;
            let observation_path = PathBuf::from(&journal.observation_path);
            let observation_id = observation_path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or("feedback journal observation filename is invalid".to_string())?;
            if !valid_observation_id(observation_id) {
                return Err("feedback journal observation id is invalid".to_string());
            }
            validate_private_target(
                &observation_path,
                &observation_directory,
                observation_id,
                false,
            )?;
            let feedback_path = PathBuf::from(&journal.feedback_path);
            validate_feedback_id(&journal.feedback_id)?;
            validate_private_target(&feedback_path, &directory, &journal.feedback_id, false)?;
            let _lock = match acquire_candidate_lock(&candidate_directory, candidate_id) {
                Ok(lock) => lock,
                Err(_) => continue,
            };
            validate_private_file(&candidate_path)?;
            self.publish_private_yaml(
                &candidate_directory,
                &candidate_path,
                &journal.original_candidate_yaml,
            )?;
            let _ = fs::remove_file(&observation_path);
            let _ = fs::remove_file(&feedback_path);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!("cannot remove recovered feedback journal: {error}"))
                }
            }
        }
        Ok(())
    }

    fn publish_private_yaml(
        &self,
        directory: &Path,
        path: &Path,
        yaml: &str,
    ) -> Result<(), String> {
        validate_private_directory(directory)?;
        if path.parent() != Some(directory) {
            return Err("private output must remain inside its destination directory".to_string());
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("private output filename is invalid".to_string())?;
        if file_name.is_empty()
            || file_name == "."
            || file_name == ".."
            || file_name.contains('/')
            || file_name.contains('\\')
        {
            return Err("private output filename is invalid".to_string());
        }
        reject_symlink_components(path)?;
        let temp_path = directory.join(format!(".tmp-{}", unix_nanos()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(&temp_path)
            .map_err(|e| format!("cannot create private temporary file: {e}"))?;
        file.write_all(yaml.as_bytes())
            .map_err(|e| format!("cannot write private temporary file: {e}"))?;
        #[cfg(unix)]
        fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("cannot set private file permissions: {e}"))?;
        fs::rename(&temp_path, path).map_err(|e| format!("cannot publish private file: {e}"))?;
        Ok(())
    }

    fn publish_private_new_yaml(
        &self,
        directory: &Path,
        path: &Path,
        yaml: &str,
    ) -> Result<(), String> {
        validate_private_directory(directory)?;
        if path.parent() != Some(directory) {
            return Err("private output must remain inside its destination directory".to_string());
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("private output filename is invalid".to_string())?;
        if file_name.is_empty()
            || file_name == "."
            || file_name == ".."
            || file_name.contains('/')
            || file_name.contains('\\')
        {
            return Err("private output filename is invalid".to_string());
        }
        reject_symlink_components(path)?;
        let temp_path = directory.join(format!(".new-{}-{}", unix_nanos(), std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(&temp_path)
            .map_err(|e| format!("cannot create private new temporary file: {e}"))?;
        file.write_all(yaml.as_bytes())
            .map_err(|e| format!("cannot write private new temporary file: {e}"))?;
        #[cfg(unix)]
        fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("cannot set private new file permissions: {e}"))?;
        let result = fs::hard_link(&temp_path, path)
            .map_err(|e| format!("cannot publish private file without overwrite: {e}"));
        let _ = fs::remove_file(&temp_path);
        result
    }

    fn observation_directory(&self) -> Result<PathBuf, String> {
        let base = env::var_os("AGENT_EXPERIENCE_DATA_ROOT")
            .or_else(|| env::var_os("MIYAGO_OBSERVATION_ROOT"))
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("XDG_DATA_HOME")
                    .map(|value| PathBuf::from(value).join("agent-experience"))
            })
            .or_else(|| {
                env::var_os("HOME")
                    .map(|value| PathBuf::from(value).join(".local/share/agent-experience"))
            })
            .ok_or("cannot determine private observation data directory".to_string())?;
        if !base.is_absolute() {
            return Err(format!(
                "observation data directory must be absolute: {}",
                base.display()
            ));
        }
        let canonical_base = canonicalize_storage_base(&base)?;
        self.reject_non_entry(&canonical_base)?;
        Ok(canonical_base.join("observations"))
    }

    fn build_route_plan(&self, query: &str, cwd: &Path, scope: &str) -> Result<RoutePlan, String> {
        let (intent, matched_signals, confidence, retrieval_order, fallback) =
            classify_route(query);
        let mut evidence = Vec::new();
        for source_id in &retrieval_order {
            let Some(source) = self
                .routing
                .knowledge_sources
                .iter()
                .find(|source| source.id == *source_id && source.enabled)
            else {
                continue;
            };
            let source_path = Path::new(&source.path);
            self.reject_non_entry(source_path)?;
            if !source_path.exists() {
                continue;
            }
            let scope_match = source.scopes.is_empty()
                || source.scopes.iter().any(|candidate| {
                    path_matches(cwd, Path::new(candidate))
                        || path_matches(Path::new(candidate), cwd)
                });
            evidence.extend(search_source(source, query, scope_match, 4)?);
        }
        evidence.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| {
                    authority_rank(&right.authority).cmp(&authority_rank(&left.authority))
                })
                .then_with(|| left.path.cmp(&right.path))
        });
        let mut seen_paths = std::collections::BTreeSet::new();
        evidence.retain(|item| seen_paths.insert(item.path.clone()));
        evidence.truncate(12);
        for item in &mut evidence {
            item.selected = item.score > 0 && item.scope_match;
        }
        Ok(RoutePlan {
            version: 1,
            intent,
            confidence,
            matched_signals,
            retrieval_order,
            source_adapter: "local_files_v1 (mcp-compatible boundary)".to_string(),
            search_policy: SearchPolicy {
                first_pass: "literal_plus_path_metadata".to_string(),
                semantic_fallback: "disabled_until_explicit_adapter_selection".to_string(),
                max_results: 12,
            },
            scope: scope.to_string(),
            evidence,
            fallback,
        })
    }

    fn assemble(&self, task: &Task) -> Result<String, String> {
        if task.context_policy.layers
            != vec![
                "task_core",
                "current_state",
                "direct_evidence",
                "background_experience",
            ]
        {
            return Err("context_policy.layers does not match required order".to_string());
        }
        if task.context_sources.len() > task.context_policy.max_items {
            return Err("context_sources count is outside policy".to_string());
        }
        let mut selected = Vec::new();
        let mut total = 0usize;
        for source in &task.context_sources {
            if !LAYERS.contains(&source.layer.as_str()) {
                return Err(format!("unsupported source layer: {}", source.layer));
            }
            let path = Path::new(&source.path);
            self.reject_non_entry(path)?;
            if !path.is_absolute() || !path.is_file() {
                return Err(format!(
                    "source is not an existing absolute file: {}",
                    path.display()
                ));
            }
            let bytes = fs::metadata(path)
                .map_err(|e| format!("cannot stat source {}: {e}", path.display()))?
                .len() as usize;
            total += bytes;
            selected.push((source, bytes));
        }
        if total > task.context_policy.max_chars {
            return Err(format!(
                "selected sources exceed max_chars: {total} > {}",
                task.context_policy.max_chars
            ));
        }
        selected.sort_by_key(|(source, _)| {
            LAYERS
                .iter()
                .position(|layer| *layer == source.layer)
                .unwrap()
        });

        let state = self.read_state(&task.task_id)?;
        let current_state = effective_text(state.as_ref(), &task.current_state);
        let decisions = effective_list(state.as_ref(), StateField::Decisions, &task.decisions);
        let blockers = effective_list(state.as_ref(), StateField::Blockers, &task.blockers);
        let next_actions =
            effective_list(state.as_ref(), StateField::NextActions, &task.next_actions);
        let mut out = format!(
            "# Context Pack: {}\n\n## Task Core\n\n- goal: {}\n- scope:\n  ~~~yaml\n",
            task.task_id, task.goal
        );
        out.push_str(&indent_yaml(&task.scope));
        out.push_str("  ~~~\n- excluded_scope:\n  ~~~yaml\n");
        out.push_str(&indent_yaml(&task.excluded_scope));
        out.push_str(
            "  ~~~\n- contamination: none\n- session_action: continue\n\n## Current State\n\n",
        );
        add_section(&mut out, "current_state", &current_state);
        add_list_section(&mut out, "verified_facts", &task.verified_facts);
        add_list_section(&mut out, "assumptions", &task.assumptions);
        add_list_section(&mut out, "decisions", &decisions);
        add_list_section(&mut out, "open_questions", &task.open_questions);
        if let Some(value) = &state {
            add_list_section(&mut out, "completed", &value.completed);
            add_list_section(&mut out, "evidence", &value.evidence);
        }
        add_list_section(&mut out, "blockers", &blockers);
        add_list_section(&mut out, "next_actions", &next_actions);
        out.push_str("## Selected\n\n");
        for (source, bytes) in &selected {
            out.push_str(&format!(
                "- `{}` [{}] ({} bytes) — {}\n",
                source.path, source.layer, bytes, source.reason
            ));
        }
        out.push_str("\n## Stop Condition\n\n");
        out.push_str(&task.stop_condition);
        out.push_str("\n\n## Selected Source Contents\n\n");
        for (source, _) in selected {
            out.push_str(&format!(
                "### {}\n\n用途：{}\n\n~~~text\n",
                source.path, source.reason
            ));
            out.push_str(
                &fs::read_to_string(&source.path)
                    .map_err(|e| format!("cannot read source {}: {e}", source.path))?,
            );
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("~~~\n\n");
        }
        Ok(format!("{}\n", out.trim_end_matches('\n')))
    }

    fn make_handoff(
        &self,
        task: &Task,
        selected: &RegistryTask,
        reason: String,
        state: Option<&TaskState>,
    ) -> Result<Handoff, String> {
        let pack_path = self.root.join(&selected.pack_path).display().to_string();
        let current_state = effective_text(state, &task.current_state);
        let completed = state
            .map(|value| value.completed.clone())
            .unwrap_or_default();
        let blockers = effective_list(state, StateField::Blockers, &task.blockers);
        let next_actions = effective_list(state, StateField::NextActions, &task.next_actions);
        Ok(Handoff {
            handoff_id: format!("{}-handoff", task.task_id),
            task_id: task.task_id.clone(),
            reason,
            summary: current_state,
            goal: task.goal.clone(),
            scope: task.scope.clone(),
            excluded_scope: task.excluded_scope.clone(),
            verified_facts: task.verified_facts.clone(),
            assumptions: task.assumptions.clone(),
            decisions: state
                .map(|value| {
                    if value.decisions.is_empty() {
                        task.decisions.clone()
                    } else {
                        value.decisions.clone()
                    }
                })
                .unwrap_or_else(|| task.decisions.clone()),
            completed,
            blockers,
            next_actions,
            artifacts: vec![pack_path],
            contamination_signals: vec![],
            stop_condition: task.stop_condition.clone(),
            do_not_reuse: vec![],
        })
    }

    fn reject_non_entry(&self, path: &Path) -> Result<(), String> {
        self.reject_non_entry_text(path)?;
        if let Ok(canonical) = fs::canonicalize(path) {
            self.reject_non_entry_text(&canonical)?;
        }
        Ok(())
    }

    fn reject_non_entry_text(&self, path: &Path) -> Result<(), String> {
        let path = path.to_string_lossy();
        for excluded in &self.routing.excluded {
            if path == excluded.path || path.starts_with(&format!("{}/", excluded.path)) {
                return Err(format!("non-entry path is forbidden: {path}"));
            }
        }
        Ok(())
    }
}

fn read_yaml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let contents =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_yaml::from_str(&contents).map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

fn load_execution_policies(workspace_root: &Path) -> Result<ExecutionPolicies, String> {
    if let Some(path) = env_setting("AGENT_EXECUTION_POLICY", "MIYAGO_EXECUTION_POLICY") {
        return read_yaml(Path::new(&path));
    }
    let local_path = workspace_root.join("records/execution-policy.yaml");
    if local_path.is_file() {
        return read_yaml(&local_path);
    }
    serde_yaml::from_str(include_str!("../config/execution-policy.yaml"))
        .map_err(|error| format!("cannot parse bundled execution policy: {error}"))
}

fn expand_config_path(value: &str, workspace_root: &Path) -> Result<String, String> {
    let home = env::var("HOME").unwrap_or_default();
    let mut expanded = value.to_string();
    let default_data_root = env::var("XDG_DATA_HOME")
        .map(|value| format!("{value}/agent-experience"))
        .unwrap_or_else(|_| format!("{home}/.local/share/agent-experience"));
    let workspace = env_setting(
        "AGENT_CONTEXT_WORKSPACE_ROOT",
        "MIYAGO_AGENT_WORKSPACE_ROOT",
    )
    .unwrap_or_else(|| workspace_root.display().to_string());
    let data_root = env_setting("AGENT_EXPERIENCE_DATA_ROOT", "MIYAGO_AGENT_DATA_ROOT")
        .unwrap_or(default_data_root);
    let defaults = [
        ("AGENT_CONTEXT_WORKSPACE_ROOT", Some(workspace.clone())),
        ("MIYAGO_AGENT_WORKSPACE_ROOT", Some(workspace)),
        (
            "AGENT_FACTORY_ROOT",
            env_setting("AGENT_FACTORY_ROOT", "MIYAGO_AGENT_FACTORY_ROOT"),
        ),
        (
            "MIYAGO_AGENT_FACTORY_ROOT",
            env_setting("AGENT_FACTORY_ROOT", "MIYAGO_AGENT_FACTORY_ROOT"),
        ),
        ("AGENT_EXPERIENCE_DATA_ROOT", Some(data_root.clone())),
        ("MIYAGO_AGENT_DATA_ROOT", Some(data_root)),
        (
            "AGENT_DOTFILE_ROOT",
            env_setting("AGENT_DOTFILE_ROOT", "MIYAGO_DOTFILE_ROOT"),
        ),
        (
            "MIYAGO_DOTFILE_ROOT",
            env_setting("AGENT_DOTFILE_ROOT", "MIYAGO_DOTFILE_ROOT"),
        ),
        (
            "AGENT_PERSONAL_VAULT_ROOT",
            env_setting("AGENT_PERSONAL_VAULT_ROOT", "MIYAGO_PERSONAL_VAULT_ROOT"),
        ),
        (
            "MIYAGO_PERSONAL_VAULT_ROOT",
            env_setting("AGENT_PERSONAL_VAULT_ROOT", "MIYAGO_PERSONAL_VAULT_ROOT"),
        ),
        (
            "AGENT_SRE_VAULT_ROOT",
            env_setting("AGENT_SRE_VAULT_ROOT", "MIYAGO_SRE_VAULT_ROOT"),
        ),
        (
            "MIYAGO_SRE_VAULT_ROOT",
            env_setting("AGENT_SRE_VAULT_ROOT", "MIYAGO_SRE_VAULT_ROOT"),
        ),
        (
            "AGENT_NON_ENTRY_ROOT",
            env_setting("AGENT_NON_ENTRY_ROOT", "MIYAGO_NON_ENTRY_ROOT"),
        ),
        (
            "MIYAGO_NON_ENTRY_ROOT",
            env_setting("AGENT_NON_ENTRY_ROOT", "MIYAGO_NON_ENTRY_ROOT"),
        ),
    ];
    for (name, replacement) in defaults {
        let token = format!("${{{name}}}");
        if expanded.contains(&token) {
            let replacement = replacement.ok_or_else(|| {
                format!("routing path requires {name}; configure it locally or remove that source")
            })?;
            expanded = expanded.replace(&token, &replacement);
        }
    }
    if expanded == "~" {
        return Ok(home);
    }
    if let Some(rest) = expanded.strip_prefix("~/") {
        expanded = format!("{home}/{rest}");
    }
    if expanded.contains("${") {
        return Err(format!("unresolved config path variable: {value}"));
    }
    Ok(expanded)
}

fn path_matches(cwd: &Path, project: &Path) -> bool {
    cwd == project || cwd.starts_with(project)
}

fn matching_tasks<'a>(tasks: &[&'a RegistryTask], cwd: &Path) -> Vec<&'a RegistryTask> {
    tasks
        .iter()
        .copied()
        .filter(|task| {
            task.projects
                .iter()
                .any(|project| path_matches(cwd, Path::new(project)))
        })
        .collect()
}

fn classify_route(query: &str) -> (String, Vec<String>, f32, Vec<String>, String) {
    let history_signals = ["以前", "過去", "之前", "歷程", "曾經", "類似", "怎麼處理"];
    let query_lower = query.to_lowercase();
    if history_signals
        .iter()
        .any(|signal| query_lower.contains(&signal.to_lowercase()))
    {
        let matched: Vec<String> = history_signals
            .iter()
            .filter(|signal| query_lower.contains(&signal.to_lowercase()))
            .map(|signal| (*signal).to_string())
            .collect();
        return (
            "collaboration_history".to_string(),
            matched,
            0.8,
            vec![
                "experience".to_string(),
                "project_wiki".to_string(),
                "raw_source".to_string(),
            ],
            "最多改寫一次查詢並補上 project 與時間訊號".to_string(),
        );
    }
    let routes: [(&str, &[&str], &[&str], &str); 5] = [
        (
            "person_understanding",
            &["我的習慣", "我的偏好", "工作方式", "使用者偏好"],
            &["personal_model", "experience", "project_wiki"],
            "只回傳已確認的個人事實",
        ),
        (
            "project_understanding",
            &["專案", "架構", "目前做到", "進度", "repo", "project"],
            &[
                "current_session",
                "project_wiki",
                "experience",
                "raw_source",
            ],
            "先限制在 project scope，找不到就明確回報未知",
        ),
        (
            "collaboration_history",
            &[
                "以前",
                "過去",
                "之前",
                "歷程",
                "類似",
                "怎麼處理",
                "怎麼處理過",
            ],
            &["experience", "project_wiki", "raw_source"],
            "最多改寫一次查詢並補上 project 與時間訊號",
        ),
        (
            "next_step",
            &["接下來", "下一步", "怎麼做", "建議", "該做什麼", "方案"],
            &[
                "current_session",
                "project_wiki",
                "experience",
                "personal_model",
            ],
            "只根據目前 task 與已選證據提出下一步",
        ),
        (
            "exact_lookup",
            &[
                "哪個檔案",
                "路徑",
                "命令",
                "原文",
                "spec",
                "設定",
                "定義在哪",
            ],
            &["project_wiki", "raw_source", "experience"],
            "優先 literal search，不足時回報未知",
        ),
    ];
    let query_lower = query.to_lowercase();
    let mut best = (
        "collaboration_history",
        Vec::new(),
        0usize,
        Vec::new(),
        "找不到足夠資料，回報未知".to_string(),
    );
    let mut total = 0usize;
    for (intent, signals, order, fallback) in routes {
        let matched: Vec<String> = signals
            .iter()
            .filter(|signal| query_lower.contains(&signal.to_lowercase()))
            .map(|signal| (*signal).to_string())
            .collect();
        total += matched.len();
        let matched_count = matched.len();
        if matched_count > best.2 {
            best = (
                intent,
                matched,
                matched_count,
                order.iter().map(|item| (*item).to_string()).collect(),
                fallback.to_string(),
            );
        }
    }
    let confidence = if total == 0 {
        0.2
    } else {
        (best.2 as f32 / total as f32).min(1.0)
    };
    (best.0.to_string(), best.1, confidence, best.3, best.4)
}

fn search_source(
    source: &KnowledgeSource,
    query: &str,
    scope_match: bool,
    limit: usize,
) -> Result<Vec<RetrievalEvidence>, String> {
    let mut terms: Vec<String> = query
        .split(|character: char| {
            character.is_whitespace() || ",.!?，。！？：:()（）[]【】".contains(character)
        })
        .filter(|term| term.chars().count() >= 2)
        .map(|term| term.to_lowercase())
        .collect();
    if terms.len() == 1 && terms[0].chars().count() > 6 {
        let characters: Vec<char> = terms[0].chars().collect();
        terms.extend(
            characters
                .windows(2)
                .map(|pair| pair.iter().collect::<String>()),
        );
    }
    let query_lower = query.to_lowercase();
    let mut files = Vec::new();
    collect_search_files(Path::new(&source.path), &mut files, 0, 256)?;
    let mut results = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&source.path)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_lowercase();
        if fs::metadata(&path)
            .map(|meta| meta.len() > 256 * 1024)
            .unwrap_or(true)
        {
            continue;
        }
        let contents = fs::read_to_string(&path).unwrap_or_default();
        let haystack = contents.to_lowercase();
        let mut score = 0usize;
        if !query_lower.is_empty()
            && (haystack.contains(&query_lower) || relative.contains(&query_lower))
        {
            score += 5;
        }
        for term in &terms {
            if haystack.contains(term) {
                score += 1;
            }
            if relative.contains(term) {
                score += 2;
            }
        }
        if score == 0 {
            continue;
        }
        let match_type = if relative.contains(&query_lower) {
            "path_metadata"
        } else if haystack.contains(&query_lower) {
            "literal_phrase"
        } else {
            "literal_term"
        };
        results.push(RetrievalEvidence {
            source_id: source.id.clone(),
            path: path.display().to_string(),
            match_type: match_type.to_string(),
            score,
            authority: source.authority.clone(),
            scope_match,
            selected: false,
            reason: format!("{} source matched the query", source.kind),
        });
    }
    results.sort_by(|left, right| right.score.cmp(&left.score));
    let mut seen_paths = std::collections::BTreeSet::new();
    results.retain(|item| seen_paths.insert(item.path.clone()));
    results.truncate(limit);
    Ok(results)
}

fn authority_rank(authority: &str) -> usize {
    match authority {
        "canonical" => 3,
        "task_state" => 2,
        "confirmed_candidate" => 2,
        "background" => 1,
        _ => 0,
    }
}

fn collect_search_files(
    root: &Path,
    files: &mut Vec<PathBuf>,
    depth: usize,
    max_files: usize,
) -> Result<(), String> {
    if files.len() >= max_files || depth > 8 {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(root)
        .map_err(|e| format!("cannot inspect search source {}: {e}", root.display()))?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_file() {
        if matches!(
            root.extension().and_then(|value| value.to_str()),
            Some("md" | "yaml" | "yml" | "toml" | "json")
        ) {
            files.push(root.to_path_buf());
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(root)
        .map_err(|e| format!("cannot read search source {}: {e}", root.display()))?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .collect();
    entries.sort();
    for path in entries {
        collect_search_files(&path, files, depth + 1, max_files)?;
        if files.len() >= max_files {
            break;
        }
    }
    Ok(())
}

fn indent_yaml<T: Serialize>(value: &T) -> String {
    let yaml = serde_yaml::to_string(value)
        .unwrap_or_default()
        .trim_start_matches("---\n")
        .to_string();
    yaml.lines().map(|line| format!("  {line}\n")).collect()
}

fn add_section(out: &mut String, name: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    out.push_str(&format!("### {name}\n\n{value}\n\n"));
}

fn add_list_section(out: &mut String, name: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    out.push_str(&format!("### {name}\n\n"));
    for value in values {
        out.push_str(&format!("- {value}\n"));
    }
    out.push('\n');
}

fn unix_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn unix_nanos() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn canonical_existing_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!("{label} must be absolute: {}", path.display()));
    }
    fs::canonicalize(path)
        .map_err(|e| format!("{label} is not accessible: {}: {e}", path.display()))
}

fn canonicalize_storage_base(base: &Path) -> Result<PathBuf, String> {
    if base.exists() {
        if fs::symlink_metadata(base)
            .map_err(|e| format!("cannot inspect observation data directory: {e}"))?
            .file_type()
            .is_symlink()
        {
            return Err(format!(
                "observation data directory cannot be a symlink: {}",
                base.display()
            ));
        }
        return fs::canonicalize(base)
            .map_err(|e| format!("cannot canonicalize observation data directory: {e}"));
    }

    let mut missing = Vec::new();
    let mut cursor = base.to_path_buf();
    while !cursor.exists() {
        let component = cursor
            .file_name()
            .ok_or("cannot find existing parent for observation data directory".to_string())?;
        missing.push(component.to_os_string());
        cursor = cursor
            .parent()
            .ok_or("cannot find existing parent for observation data directory".to_string())
            .map(Path::to_path_buf)?;
    }
    let mut result = fs::canonicalize(&cursor)
        .map_err(|e| format!("cannot canonicalize observation data parent: {e}"))?;
    for component in missing.iter().rev() {
        result.push(component);
    }
    Ok(result)
}

fn reject_symlink_components(path: &Path) -> Result<(), String> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if let Ok(metadata) = fs::symlink_metadata(&current) {
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "observation path contains symlink component: {}",
                    current.display()
                ));
            }
        }
    }
    Ok(())
}

fn validate_observation_enum(name: &str, value: &str, allowed: &[&str]) -> Result<(), String> {
    validate_observation_text(name, value, 80)?;
    if !allowed.contains(&value) {
        return Err(format!(
            "unsupported observation {name}: {value}; expected one of {}",
            allowed.join(", ")
        ));
    }
    Ok(())
}

fn has_only_jev_reviews(history: &[DecisionRecord]) -> bool {
    !history.is_empty() && history.iter().all(|entry| entry.decision == "jev_review")
}

fn jev_review_is_current(
    decision_history: &[DecisionRecord],
    reviewed_observation_count: usize,
    observation_count: usize,
) -> bool {
    decision_history
        .iter()
        .any(|entry| entry.decision == "jev_review")
        && reviewed_observation_count >= observation_count
}

fn candidate_group_key(event: &ObservationEvent) -> (String, Vec<String>) {
    let mut normalized = event.summary.to_lowercase();
    normalized.retain(|character| !character.is_whitespace());
    let mut scope = event.scope.clone();
    scope.sort();
    scope.dedup();
    (normalized, scope)
}

fn candidate_group_material(summary: &str, original_scope: &[String]) -> (String, Vec<String>) {
    let mut normalized = summary.to_lowercase();
    normalized.retain(|character| !character.is_whitespace());
    let mut scope = original_scope.to_vec();
    scope.sort();
    scope.dedup();
    (normalized, scope)
}

fn remap_import_scope(scope: &str, mappings: &[(PathBuf, PathBuf)]) -> Result<String, String> {
    let original = Path::new(scope);
    for (old, new) in mappings {
        if original == old || original.starts_with(old) {
            let relative = original
                .strip_prefix(old)
                .map_err(|e| format!("cannot remap import scope {scope}: {e}"))?;
            let mapped = if relative.as_os_str().is_empty() {
                new.clone()
            } else {
                new.join(relative)
            };
            return Ok(mapped.display().to_string());
        }
    }
    if original.exists() {
        return Ok(canonical_existing_path(original, "import scope")
            .map(|path| path.display().to_string())?);
    }
    Err(format!(
        "import scope does not exist and has no --map: {scope}"
    ))
}

fn candidate_key_material(key: &(String, Vec<String>)) -> String {
    let mut material = format!("{}:", key.0.len());
    material.push_str(&key.0);
    for scope in &key.1 {
        material.push_str(&format!("{}:", scope.len()));
        material.push_str(scope);
    }
    material
}

fn scopes_overlap(left: &str, right: &str) -> bool {
    let left = Path::new(left);
    let right = Path::new(right);
    left.starts_with(right) || right.starts_with(left)
}

fn valid_observation_id(value: &str) -> bool {
    let mut parts = value.split('-');
    parts.next() == Some("obs")
        && parts.next().is_some_and(|part| {
            (1..=20).contains(&part.len()) && part.chars().all(|c| c.is_ascii_digit())
        })
        && parts.next().is_some_and(|part| {
            (1..=10).contains(&part.len()) && part.chars().all(|c| c.is_ascii_digit())
        })
        && parts.next().is_none()
}

fn valid_timestamp(value: &str) -> bool {
    (1..=20).contains(&value.len())
        && value.chars().all(|character| character.is_ascii_digit())
        && value
            .parse::<u64>()
            .map(|number| number > 0)
            .unwrap_or(false)
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 14695981039346656037u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn validate_task_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 80
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
    {
        return Err(format!("task id is not a safe filename segment: {value}"));
    }
    Ok(())
}

#[cfg(unix)]
fn acquire_candidate_lock(directory: &Path, candidate_id: &str) -> Result<CandidateLock, String> {
    validate_candidate_id(candidate_id)?;
    validate_private_directory(directory)?;
    let path = directory.join(format!(".{candidate_id}.lock"));
    reject_symlink_components(&path)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).mode(0o600);
    let mut file = options
        .open(&path)
        .map_err(|e| format!("cannot open candidate lock: {e}"))?;
    if unsafe { flock(file.as_raw_fd(), 2 | 4) } != 0 {
        return Err("candidate is locked by another process".to_string());
    }
    file.set_len(0)
        .map_err(|e| format!("cannot reset candidate lock metadata: {e}"))?;
    file.write_all(format!("{} {}", std::process::id(), unix_timestamp()).as_bytes())
        .map_err(|e| format!("cannot write candidate lock metadata: {e}"))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("cannot set candidate lock permissions: {e}"))?;
    Ok(CandidateLock { _file: file })
}

#[cfg(not(unix))]
fn acquire_candidate_lock(directory: &Path, candidate_id: &str) -> Result<CandidateLock, String> {
    validate_candidate_id(candidate_id)?;
    validate_private_directory(directory)?;
    Ok(CandidateLock {})
}

fn validate_candidate_id(value: &str) -> Result<(), String> {
    if !value.starts_with("candidate-")
        || value.len() > 80
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("invalid candidate id".to_string());
    }
    Ok(())
}

fn validate_feedback_id(value: &str) -> Result<(), String> {
    if !value.starts_with("feedback-")
        || value.len() > 80
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("invalid feedback id".to_string());
    }
    Ok(())
}

fn validate_private_target(
    path: &Path,
    directory: &Path,
    expected_stem: &str,
    must_exist: bool,
) -> Result<(), String> {
    if path.parent() != Some(directory) {
        return Err("private target escaped its directory".to_string());
    }
    if path.file_stem().and_then(|value| value.to_str()) != Some(expected_stem) {
        return Err("private target identity does not match its filename".to_string());
    }
    reject_symlink_components(path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => validate_private_file(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !must_exist => Ok(()),
        Err(error) => Err(format!("cannot inspect private target: {error}")),
    }
}

fn validate_owner(value: &str) -> Result<(), String> {
    if ![
        "personal_model",
        "shared_contract",
        "project_knowledge",
        "task_record",
        "experience_library",
    ]
    .contains(&value)
    {
        return Err(format!("unsupported experience owner: {value}"));
    }
    Ok(())
}

fn is_strictly_narrower(candidate_scope: &[String], previous_scope: &[String]) -> bool {
    if candidate_scope.is_empty() || previous_scope.is_empty() {
        return false;
    }
    let mut new_scope = candidate_scope.to_vec();
    let mut old_scope = previous_scope.to_vec();
    new_scope.sort();
    new_scope.dedup();
    old_scope.sort();
    old_scope.dedup();
    new_scope != old_scope
        && new_scope.iter().all(|candidate| {
            old_scope.iter().any(|previous| {
                candidate != previous && candidate.starts_with(&format!("{previous}/"))
            })
        })
}

#[cfg(unix)]
fn validate_private_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect private directory {}: {e}", path.display()))?;
    if !metadata.file_type().is_dir() {
        return Err(format!(
            "private storage is not a directory: {}",
            path.display()
        ));
    }
    if metadata.uid() != unsafe { geteuid() } {
        return Err(format!(
            "private storage owner mismatch: {}",
            path.display()
        ));
    }
    if metadata.mode() & 0o077 != 0 {
        return Err(format!(
            "private storage permissions are too broad: {}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn validate_private_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect private file {}: {e}", path.display()))?;
    if !metadata.file_type().is_file() {
        return Err(format!(
            "private storage is not a regular file: {}",
            path.display()
        ));
    }
    if metadata.uid() != unsafe { geteuid() } {
        return Err(format!("private file owner mismatch: {}", path.display()));
    }
    if metadata.mode() & 0o077 != 0 {
        return Err(format!(
            "private file permissions are too broad: {}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_private_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(unix))]
fn validate_private_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn validate_observation_text(name: &str, value: &str, max_chars: usize) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("observation {name} cannot be empty"));
    }
    if value.chars().count() > max_chars {
        return Err(format!("observation {name} exceeds {max_chars} characters"));
    }
    if value != value.trim()
        || value.chars().any(|character| character.is_control())
        || contains_sensitive_shape(value)
    {
        return Err(format!(
            "observation {name} contains whitespace/control/credential shape and was rejected"
        ));
    }
    Ok(())
}

fn contains_sensitive_shape(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("begin private key")
        || lower.contains("authorization:")
        || lower.contains("bearer ")
        || lower.contains("cookie:")
        || lower.contains("set-cookie:")
        || lower.contains("aws_access_key_id")
        || lower.contains("aws_secret_access_key")
        || lower.contains("x-api-key:")
        || lower.contains("api_key=")
        || lower.contains("api-key=")
        || lower.contains("token=")
        || lower.contains("secret=")
        || lower.contains("ghp_")
        || lower.contains("sk-")
        || lower.contains("xoxb-")
}

fn append_unique(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

fn effective_text(state: Option<&TaskState>, fallback: &str) -> String {
    state
        .filter(|value| !value.current_state.is_empty())
        .map(|value| value.current_state.clone())
        .unwrap_or_else(|| fallback.to_string())
}

enum StateField {
    Blockers,
    NextActions,
    Decisions,
}

fn effective_list(
    state: Option<&TaskState>,
    field: StateField,
    fallback: &[String],
) -> Vec<String> {
    let values = state.and_then(|value| match field {
        StateField::Blockers if !value.blockers.is_empty() => Some(value.blockers.clone()),
        StateField::NextActions if !value.next_actions.is_empty() => {
            Some(value.next_actions.clone())
        }
        StateField::Decisions if !value.decisions.is_empty() => Some(value.decisions.clone()),
        _ => None,
    });
    values.unwrap_or_else(|| fallback.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jev_review_is_reused_until_new_observation_arrives() {
        let history = vec![DecisionRecord {
            decision: "jev_review".to_string(),
            at: "1".to_string(),
            reason: "low confidence".to_string(),
        }];
        assert!(jev_review_is_current(&history, 2, 2));
        assert!(!jev_review_is_current(&history, 2, 3));
        assert!(!jev_review_is_current(&[], 2, 2));
    }

    #[test]
    fn only_jev_reviews_may_be_reopened_by_new_evidence() {
        let jev_only = vec![DecisionRecord {
            decision: "jev_review".to_string(),
            at: "1".to_string(),
            reason: "review".to_string(),
        }];
        let mixed = vec![
            jev_only[0].clone(),
            DecisionRecord {
                decision: "confirm".to_string(),
                at: "2".to_string(),
                reason: "human decision".to_string(),
            },
        ];
        assert!(has_only_jev_reviews(&jev_only));
        assert!(!has_only_jev_reviews(&mixed));
        assert!(!has_only_jev_reviews(&[]));
    }

    #[test]
    fn execution_policy_requires_phase4_fields() {
        let complete = r#"
version: 1
default_profile: review
profiles:
  review:
    effort: medium
    model: review-reasoning
    verification: fixed rubric
    human_gate: before delivery
    allowed_actions: [read]
    skills: [code-review]
    tools: [filesystem-read]
    delegation: bounded-readonly
    notes: review only
"#;
        for field in [
            "    model: review-reasoning\n",
            "    skills: [code-review]\n",
            "    tools: [filesystem-read]\n",
            "    delegation: bounded-readonly\n",
            "    allowed_actions: [read]\n",
        ] {
            let missing = complete.replace(field, "");
            assert!(
                serde_yaml::from_str::<ExecutionPolicies>(&missing).is_err(),
                "missing field should fail closed: {field}"
            );
        }
        assert!(serde_yaml::from_str::<ExecutionPolicies>(complete).is_ok());
    }

    #[test]
    fn write_capable_profiles_keep_high_risk_human_gates() {
        let policies = load_execution_policies(Path::new("/tmp/nonexistent-workspace")).unwrap();
        for profile_name in ["implementation", "debugging", "operations"] {
            let profile = policies.profiles.get(profile_name).unwrap();
            let gate = profile.human_gate.to_lowercase();
            for required in ["production", "credential", "destructive", "scope expansion"] {
                assert!(
                    gate.contains(required),
                    "{profile_name} gate is missing {required}: {}",
                    profile.human_gate
                );
            }
        }
    }

    #[test]
    fn planning_task_state_write_set_matches_checkpoint() {
        let policies = load_execution_policies(Path::new("/tmp/nonexistent-workspace")).unwrap();
        let profile = policies.profiles.get("planning").unwrap();
        let actions = profile
            .allowed_actions
            .iter()
            .find(|action| action.starts_with("透過 checkpoint"))
            .expect("planning checkpoint action");
        for field in [
            "current_state",
            "completed",
            "next_actions",
            "blockers",
            "decisions",
            "evidence",
        ] {
            assert!(actions.contains(field), "planning write set misses {field}");
        }
        for forbidden in [
            "scope",
            "excluded scope",
            "Personal Model",
            "open_questions",
        ] {
            assert!(
                !actions.contains(forbidden),
                "planning write set must exclude {forbidden}"
            );
        }
        assert!(profile.tools.iter().any(|tool| tool == "task-state-write"));
    }

    #[test]
    fn new_project_work_gets_a_safe_local_task_id() {
        let task_id = auto_task_id(Path::new("/tmp/Argocd_Config"));
        assert!(task_id.starts_with("auto-argocd-config-"));
        assert!(validate_task_id(&task_id).is_ok());
    }

    #[test]
    fn only_workflow_entry_commands_create_local_tasks() {
        assert!(command_creates_local_task("plan"));
        assert!(command_creates_local_task("session-start"));
        assert!(!command_creates_local_task("route"));
        assert!(!command_creates_local_task("status"));
    }

    fn registry_task(task_id: &str, projects: &[&str]) -> RegistryTask {
        RegistryTask {
            task_id: task_id.to_string(),
            status: "active".to_string(),
            task_path: format!("records/tasks/{task_id}.yaml"),
            pack_path: format!("records/contexts/{task_id}-context-pack.md"),
            projects: projects.iter().map(|value| value.to_string()).collect(),
            preferred_runtimes: vec![],
        }
    }

    #[test]
    fn observation_text_rejects_transcript_shape() {
        let error = validate_observation_text("summary", "第一行\n第二行", 100).unwrap_err();
        assert!(error.contains("control"));
    }

    #[test]
    fn observation_text_rejects_oversized_input() {
        let error = validate_observation_text("summary", &"x".repeat(11), 10).unwrap_err();
        assert!(error.contains("exceeds 10"));
    }

    #[test]
    fn observation_text_rejects_credential_shapes() {
        assert!(validate_observation_text("summary", "Bearer canary", 100).is_err());
        assert!(validate_observation_text("summary", "-----BEGIN PRIVATE KEY-----", 100).is_err());
        assert!(validate_observation_text("summary", "safe engineering preference", 100).is_ok());
    }

    #[test]
    fn observation_enum_rejects_unknown_provenance() {
        assert!(validate_observation_enum("runtime", "unknown", &["codex", "claude"]).is_err());
        assert!(validate_observation_enum("runtime", "codex", &["codex", "claude"]).is_ok());
    }

    #[test]
    fn task_and_candidate_ids_are_safe_filename_segments() {
        assert!(validate_task_id("experience-autocapture").is_ok());
        assert!(validate_task_id("../outside").is_err());
        assert!(validate_task_id("/tmp/outside").is_err());
        assert!(validate_candidate_id("candidate-0123").is_ok());
        assert!(validate_candidate_id("candidate-../outside").is_err());
    }

    #[test]
    fn candidate_lock_serializes_updates() {
        let directory = fs::canonicalize(env::temp_dir())
            .unwrap()
            .join(format!("agent-experience-lock-{}", unix_nanos()));
        fs::create_dir(&directory).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let first = acquire_candidate_lock(&directory, "candidate-lock-test").unwrap();
        assert!(acquire_candidate_lock(&directory, "candidate-lock-test").is_err());
        drop(first);
        let second = acquire_candidate_lock(&directory, "candidate-lock-test").unwrap();
        drop(second);
        fs::remove_file(directory.join(".candidate-lock-test.lock")).unwrap();
        fs::remove_dir(&directory).unwrap();
    }

    #[test]
    fn narrowing_requires_a_strict_descendant_scope() {
        let old = vec!["/tmp/project".to_string()];
        assert!(is_strictly_narrower(
            &["/tmp/project/src".to_string()],
            &old
        ));
        assert!(!is_strictly_narrower(&["/tmp/project".to_string()], &old));
        assert!(!is_strictly_narrower(&["/tmp".to_string()], &old));
        assert!(!is_strictly_narrower(&["/tmp/other".to_string()], &old));
    }

    #[test]
    fn matching_tasks_returns_single_cross_project_task() {
        let task = registry_task("cross-project", &["/tmp/project-a", "/tmp/project-b"]);
        let refs = vec![&task];
        let found = matching_tasks(&refs, Path::new("/tmp/project-b/services/api"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].task_id, "cross-project");
    }

    #[test]
    fn matching_tasks_keeps_ambiguity_for_caller_to_reject() {
        let first = registry_task("first", &["/tmp/shared"]);
        let second = registry_task("second", &["/tmp/shared"]);
        let refs = vec![&first, &second];
        let found = matching_tasks(&refs, Path::new("/tmp/shared/repo"));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn non_entry_is_rejected() {
        let harness = Harness {
            root: PathBuf::from("/tmp/workspace"),
            routing: Routing {
                excluded: vec![Excluded {
                    path: "/tmp/forbidden".to_string(),
                }],
                knowledge_sources: vec![],
            },
            registry: Registry {
                version: 1,
                tasks: vec![],
            },
        };
        assert!(harness
            .reject_non_entry(Path::new("/tmp/forbidden/project"))
            .is_err());
        assert!(harness
            .reject_non_entry(Path::new("/tmp/allowed/project"))
            .is_ok());
    }

    #[test]
    fn config_paths_expand_from_environment_and_workspace_root() {
        let expanded = expand_config_path(
            "${AGENT_CONTEXT_WORKSPACE_ROOT}/records",
            Path::new("/tmp/example-workspace"),
        )
        .unwrap();
        assert!(expanded.ends_with("/records"));
        assert!(!expanded.contains("${"));
    }

    #[test]
    fn discover_rejects_multiple_active_tasks() {
        let task_path = "/tmp/context-harness-test/agent-workspace/records/tasks/parity-check.yaml";
        let first = RegistryTask {
            task_id: "first".to_string(),
            status: "active".to_string(),
            task_path: task_path
                .strip_prefix("/tmp/context-harness-test/agent-workspace/")
                .unwrap()
                .to_string(),
            pack_path: "records/contexts/first.md".to_string(),
            projects: vec!["/tmp/context-harness-test/agent-workspace".to_string()],
            preferred_runtimes: vec![],
        };
        let second = RegistryTask {
            task_id: "second".to_string(),
            status: "active".to_string(),
            task_path: task_path
                .strip_prefix("/tmp/context-harness-test/agent-workspace/")
                .unwrap()
                .to_string(),
            pack_path: "records/contexts/second.md".to_string(),
            projects: vec!["/tmp/context-harness-test/agent-workspace".to_string()],
            preferred_runtimes: vec![],
        };
        let harness = Harness {
            root: PathBuf::from("/tmp/context-harness-test/agent-workspace"),
            routing: Routing {
                excluded: vec![Excluded {
                    path: "/tmp/context-harness-test/non-entry".to_string(),
                }],
                knowledge_sources: vec![],
            },
            registry: Registry {
                version: 1,
                tasks: vec![first, second],
            },
        };
        let error = harness
            .discover(
                None,
                Path::new("/tmp/context-harness-test/agent-workspace"),
                false,
            )
            .unwrap_err();
        assert!(error.contains("multiple active tasks"));
    }

    #[test]
    fn assemble_rejects_non_entry_source_before_reading_it() {
        let harness = Harness {
            root: PathBuf::from("/tmp/context-harness-test/agent-workspace"),
            routing: Routing {
                excluded: vec![Excluded {
                    path: "/tmp/context-harness-test/non-entry".to_string(),
                }],
                knowledge_sources: vec![],
            },
            registry: Registry {
                version: 1,
                tasks: vec![],
            },
        };
        let task = Task {
            task_id: "contaminated".to_string(),
            profile: "review".to_string(),
            goal: "test".to_string(),
            scope: serde_yaml::from_str("{}\n").unwrap(),
            excluded_scope: vec!["/tmp/context-harness-test/non-entry".to_string()],
            stop_condition: "stop".to_string(),
            current_state: String::new(),
            verified_facts: vec![],
            assumptions: vec![],
            decisions: vec![],
            open_questions: vec![],
            blockers: vec![],
            next_actions: vec![],
            context_sources: vec![ContextSource {
                path: "/tmp/context-harness-test/non-entry/should-not-read.md".to_string(),
                layer: "direct_evidence".to_string(),
                reason: "contamination".to_string(),
            }],
            context_policy: ContextPolicy {
                layers: vec![
                    "task_core".to_string(),
                    "current_state".to_string(),
                    "direct_evidence".to_string(),
                    "background_experience".to_string(),
                ],
                max_items: 1,
                max_chars: 1000,
            },
        };
        let error = harness.assemble(&task).unwrap_err();
        assert!(error.contains("non-entry path is forbidden"));
    }
}
