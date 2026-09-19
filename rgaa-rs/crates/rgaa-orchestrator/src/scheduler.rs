//! In-process recurring runs (nightly by default) of the combined RGAA +
//! SEO/GEO/AEO audit, feeding the remediation lifecycle.
//!
//! A run audits the job's URLs through the regular [`Orchestrator`] pipeline,
//! folds SEO findings onto the RGAA ones on the P0–P3 ladder, opens a
//! [`FindingLifecycle`] per finding, and — when the job names the template
//! that owns the document `<head>` — asks [`HtmlAdapter`] for patch
//! proposals. Everything a run produces stays a proposal: approval is
//! forced on, nothing is ever applied to a page or a listing.

use crate::Orchestrator;
use async_trait::async_trait;
use chrono::{DateTime, Local, TimeZone};
use indexmap::IndexMap;
use rgaa_core::{AuditBundle, AuditResult, CrawlConfig, CriterionResult, EvidenceRef};
use rgaa_remediation::{
    issue_from_finding, merge_with_rgaa, remediate, seo_findings, FindingLifecycle, FindingState,
    HtmlAdapter, MergedFinding, RemediationErrorCode, RemediationIssue, RemediationOutcome,
    RemediationPolicy, SourceLocation,
};
use rgaa_storage::Storage;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

pub const SCHEDULER_ACTOR: &str = "scheduler";

/// When a job runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Schedule {
    /// Every day at the given local wall-clock time.
    Daily { hour: u32, minute: u32 },
    /// At a fixed interval from the previous run.
    Every(Duration),
}

impl Schedule {
    /// 02:00 local time — after most content updates, before the working day.
    pub const NIGHTLY: Schedule = Schedule::Daily { hour: 2, minute: 0 };

    /// First occurrence strictly after `now`.
    #[must_use]
    pub fn next_run(&self, now: DateTime<Local>) -> DateTime<Local> {
        match self {
            Schedule::Every(interval) => {
                now + chrono::Duration::from_std(*interval).unwrap_or(chrono::Duration::MAX)
            }
            Schedule::Daily { hour, minute } => {
                let mut date = now.date_naive();
                loop {
                    let candidate = date
                        .and_hms_opt(*hour, *minute, 0)
                        .expect("hour and minute validated at construction");
                    // Ambiguous or skipped local times (DST) resolve to the
                    // earliest valid instant, else the next day.
                    if let Some(at) = Local.from_local_datetime(&candidate).earliest() {
                        if at > now {
                            return at;
                        }
                    }
                    date = date.succ_opt().expect("date range not exhausted");
                }
            }
        }
    }
}

/// A recurring combined audit.
#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub name: String,
    pub urls: Vec<String>,
    pub crawl: CrawlConfig,
    pub schedule: Schedule,
    /// Source file that renders the document `<head>` (layout template). Read
    /// at run time and handed to [`HtmlAdapter`] so proposals are real diffs
    /// against the real file. `None` leaves findings triaged without patches.
    pub head_template: Option<PathBuf>,
    pub policy: RemediationPolicy,
}

impl ScheduledJob {
    /// Nightly job with default crawl and policy.
    #[must_use]
    pub fn nightly(name: impl Into<String>, urls: Vec<String>) -> Self {
        Self {
            name: name.into(),
            urls,
            crawl: CrawlConfig::default(),
            schedule: Schedule::NIGHTLY,
            head_template: None,
            policy: RemediationPolicy::default(),
        }
    }

    #[must_use]
    pub fn with_head_template(mut self, path: impl Into<PathBuf>) -> Self {
        self.head_template = Some(path.into());
        self
    }

    #[must_use]
    pub fn with_schedule(mut self, schedule: Schedule) -> Self {
        self.schedule = schedule;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("job name is empty".into());
        }
        if self.urls.is_empty() {
            return Err(format!("job {}: no urls", self.name));
        }
        if let Schedule::Daily { hour, minute } = self.schedule {
            if hour > 23 || minute > 59 {
                return Err(format!(
                    "job {}: invalid time {hour:02}:{minute:02}",
                    self.name
                ));
            }
        }
        if let Schedule::Every(d) = self.schedule {
            if d < Duration::from_secs(60) {
                return Err(format!("job {}: interval below one minute", self.name));
            }
        }
        Ok(())
    }
}

/// Everything one execution of a job produced.
#[derive(Debug, Clone)]
pub struct ScheduledRun {
    pub job: String,
    pub started_at: DateTime<Local>,
    pub finished_at: DateTime<Local>,
    pub audits: Vec<AuditResult>,
    /// RGAA + SEO findings, P0 first.
    pub findings: Vec<MergedFinding>,
    /// One lifecycle per finding, in the same order as `findings`.
    pub lifecycles: Vec<FindingLifecycle>,
    pub proposals: Vec<RemediationOutcome>,
}

impl ScheduledRun {
    pub fn count_in(&self, state: FindingState) -> usize {
        self.lifecycles.iter().filter(|l| l.state == state).count()
    }

    pub fn summary_json(&self) -> serde_json::Value {
        serde_json::json!({
            "job": self.job,
            "started_at": self.started_at.to_rfc3339(),
            "finished_at": self.finished_at.to_rfc3339(),
            "audits": self.audits.len(),
            "findings": self.findings.len(),
            "by_criticality": {
                "P0": self.findings.iter().filter(|f| f.criticality.as_str() == "P0").count(),
                "P1": self.findings.iter().filter(|f| f.criticality.as_str() == "P1").count(),
                "P2": self.findings.iter().filter(|f| f.criticality.as_str() == "P2").count(),
                "P3": self.findings.iter().filter(|f| f.criticality.as_str() == "P3").count(),
            },
            "awaiting_approval": self.count_in(FindingState::AwaitingApproval),
            "needs_review": self.count_in(FindingState::NeedsReview),
            "triaged": self.count_in(FindingState::Triaged),
            "proposals": self.proposals.len(),
        })
    }
}

/// Where a finished run goes.
#[async_trait]
pub trait RunSink: Send + Sync {
    async fn deliver(&self, run: &ScheduledRun) -> Result<(), String>;
}

/// Logs the run summary; the default sink.
pub struct LogSink;

#[async_trait]
impl RunSink for LogSink {
    async fn deliver(&self, run: &ScheduledRun) -> Result<(), String> {
        info!(summary = %run.summary_json(), "scheduled run complete");
        Ok(())
    }
}

/// Appends the run summary to the audit log of every audit the run produced
/// (the audits themselves are persisted by the pipeline as they complete).
pub struct StorageSink(pub Arc<dyn Storage>);

#[async_trait]
impl RunSink for StorageSink {
    async fn deliver(&self, run: &ScheduledRun) -> Result<(), String> {
        let details = run.summary_json();
        for audit in &run.audits {
            self.0
                .save_audit_log(&audit.audit_id, "scheduled-run", Some(details.clone()))
                .await
                .map_err(|e| format!("audit log for {}: {e}", audit.audit_id))?;
        }
        Ok(())
    }
}

/// Pure planning step: audits in, lifecycle-tracked findings and proposals out.
/// Separated from I/O so it is testable without a browser or model server.
pub fn plan_run(
    job: &ScheduledJob,
    mut audits: Vec<AuditResult>,
    head_html: Option<&str>,
    started_at: DateTime<Local>,
) -> ScheduledRun {
    audits.sort_by(|a, b| a.url.cmp(&b.url));

    let mut rgaa = Vec::new();
    let mut seo = Vec::new();
    for audit in &audits {
        let evidence = vec![EvidenceRef::new("audit", audit.audit_id.clone())];
        for page in &audit.pages {
            let results: IndexMap<String, CriterionResult> = page
                .seo
                .iter()
                .map(|r| (r.criterion_id.clone(), r.clone()))
                .collect();
            seo.extend(seo_findings(&results, &page.url, &evidence));
        }
        rgaa.extend(AuditBundle::from(audit.clone()).findings);
    }

    let findings = merge_with_rgaa(&rgaa, &seo);
    let reason = |m: &MergedFinding| {
        format!(
            "{}: {} run, origins {}",
            m.criticality.as_str(),
            job.name,
            m.origins.join("+")
        )
    };
    let mut lifecycles: Vec<FindingLifecycle> = findings
        .iter()
        .map(|m| {
            let mut lc = FindingLifecycle::new(m.finding.id.clone());
            lc.transition(FindingState::Triaged, SCHEDULER_ACTOR, &reason(m))
                .expect("Open -> Triaged is a valid transition");
            lc
        })
        .collect();

    let mut proposals = Vec::new();
    if let (Some(head_html), Some(template)) = (head_html, &job.head_template) {
        // Scheduled output is reviewed before anything is applied, whatever
        // the job's policy says.
        let mut policy = job.policy.clone();
        policy.require_approval = true;

        let location = SourceLocation {
            file: template.to_string_lossy().into_owned(),
            line: 1,
            column: None,
        };
        let issues: Vec<RemediationIssue> = findings
            .iter()
            .filter(|m| is_seo_rule(&m.finding.rule))
            .map(|m| issue_from_finding(&m.finding, head_html, location.clone()))
            .collect();

        let batch_size = policy.max_batch_size.clamp(1, 25);
        for batch in issues.chunks(batch_size) {
            match remediate(batch, &policy, &HtmlAdapter) {
                Ok(outcomes) => proposals.extend(outcomes),
                Err(e) => warn!(job = job.name, error = %e, "remediation batch rejected"),
            }
        }

        for outcome in &proposals {
            let (id, next, why) = match outcome {
                RemediationOutcome::Ok(g) => (
                    &g.issue_id,
                    Some(FindingState::AwaitingApproval),
                    "patch proposed by scheduler; approval required".to_string(),
                ),
                RemediationOutcome::Error(e) if e.code == RemediationErrorCode::NeedsReview => (
                    &e.issue_id,
                    Some(FindingState::NeedsReview),
                    e.message.clone(),
                ),
                RemediationOutcome::Error(e) => (&e.issue_id, None, e.message.clone()),
            };
            let Some(lc) = lifecycles.iter_mut().find(|l| &l.finding_id == id) else {
                continue;
            };
            match next {
                Some(FindingState::AwaitingApproval) => {
                    let _ = lc.transition(FindingState::FixProposed, SCHEDULER_ACTOR, &why);
                    let _ = lc.transition(FindingState::AwaitingApproval, SCHEDULER_ACTOR, &why);
                }
                Some(state) => {
                    let _ = lc.transition(state, SCHEDULER_ACTOR, &why);
                }
                None => {
                    warn!(finding = %id, reason = %why, "proposal not produced; finding stays triaged")
                }
            }
        }
    } else if head_html.is_none() && job.head_template.is_some() {
        warn!(
            job = job.name,
            "head template unreadable; no patches proposed"
        );
    }

    ScheduledRun {
        job: job.name.clone(),
        started_at,
        finished_at: Local::now(),
        audits,
        findings,
        lifecycles,
        proposals,
    }
}

fn is_seo_rule(rule: &str) -> bool {
    rule.starts_with("SEO-") || rule.starts_with("GEO-") || rule.starts_with("AEO-")
}

/// Drives jobs on their schedule against a shared [`Orchestrator`].
pub struct Scheduler {
    orchestrator: Arc<Orchestrator>,
    sink: Arc<dyn RunSink>,
}

impl Scheduler {
    pub fn new(orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            orchestrator,
            sink: Arc::new(LogSink),
        }
    }

    #[must_use]
    pub fn with_sink(mut self, sink: Arc<dyn RunSink>) -> Self {
        self.sink = sink;
        self
    }

    /// Executes one job now: audit, plan, deliver.
    pub async fn run_once(&self, job: &ScheduledJob) -> Result<ScheduledRun, String> {
        job.validate()?;
        let started_at = Local::now();
        info!(
            job = job.name,
            urls = job.urls.len(),
            "scheduled run starting"
        );

        let audits = self.orchestrator.run_batch(&job.urls, &job.crawl).await?;
        if audits.is_empty() {
            return Err(format!("job {}: every audit failed", job.name));
        }

        let head_html = match &job.head_template {
            Some(path) => match tokio::fs::read_to_string(path).await {
                Ok(html) => Some(html),
                Err(e) => {
                    warn!(job = job.name, path = %path.display(), error = %e, "cannot read head template");
                    None
                }
            },
            None => None,
        };

        let run = plan_run(
            job,
            audits.into_values().collect(),
            head_html.as_deref(),
            started_at,
        );
        self.sink.deliver(&run).await?;
        Ok(run)
    }

    /// Runs every job on its schedule until `stop` flips to `true`.
    /// A failed run is logged and the job is rescheduled; it never ends the loop.
    pub async fn run_until(
        &self,
        jobs: Vec<ScheduledJob>,
        mut stop: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), String> {
        for job in &jobs {
            job.validate()?;
        }
        if jobs.is_empty() {
            return Err("no jobs to schedule".into());
        }

        let mut next: Vec<DateTime<Local>> = {
            let now = Local::now();
            jobs.iter().map(|j| j.schedule.next_run(now)).collect()
        };

        loop {
            if *stop.borrow() {
                return Ok(());
            }
            let (idx, at) = next
                .iter()
                .enumerate()
                .min_by_key(|(_, at)| **at)
                .map(|(i, at)| (i, *at))
                .expect("jobs is non-empty");
            let wait = (at - Local::now()).to_std().unwrap_or(Duration::ZERO);
            info!(job = jobs[idx].name, at = %at.to_rfc3339(), "next scheduled run");

            tokio::select! {
                _ = tokio::time::sleep(wait) => {
                    if let Err(e) = self.run_once(&jobs[idx]).await {
                        warn!(job = jobs[idx].name, error = %e, "scheduled run failed");
                    }
                    next[idx] = jobs[idx].schedule.next_run(Local::now());
                }
                changed = stop.changed() => {
                    if changed.is_err() || *stop.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
    use rgaa_core::{Classification, CriterionStatus, PageResult, Violation};
    use rgaa_rules::seo::{PageSnapshot, SeoMapper};

    const URL: &str = "https://example.test/";

    fn nightly_job() -> ScheduledJob {
        ScheduledJob::nightly("nightly-seo", vec![URL.into()])
    }

    fn rgaa_result(id: &str, status: CriterionStatus, impact: &str) -> CriterionResult {
        CriterionResult {
            criterion_id: id.into(),
            title: id.into(),
            classification: Classification::Deterministe,
            status: status.clone(),
            violations: if status == CriterionStatus::Fail {
                vec![Violation {
                    rule_id: format!("axe-{id}"),
                    impact: impact.into(),
                    description: format!("{id} failed"),
                    nodes_affected: 1,
                }]
            } else {
                vec![]
            },
            confidence: None,
            justification: None,
            source: "axe-core".into(),
        }
    }

    fn audit(snapshot: PageSnapshot, rgaa: Vec<CriterionResult>) -> AuditResult {
        let seo: Vec<CriterionResult> =
            SeoMapper::evaluate(&snapshot, None).into_values().collect();
        AuditResult {
            audit_id: "audit-1".into(),
            url: URL.into(),
            pages: vec![PageResult {
                url: URL.into(),
                title: None,
                criteria: rgaa,
                compliance_rate: 0.0,
                crawl_depth: 0,
                seo,
            }],
            total_criteria: 106,
            passed: 0,
            failed: 0,
            na: 0,
            overall_compliance: 0.0,
            taux_global: 0.0,
            coverage_percent: 0.0,
            etat_conformite: "non conforme".into(),
            duration_ms: 1,
        }
    }

    fn noindex_page() -> PageSnapshot {
        PageSnapshot {
            url: URL.into(),
            lang: Some("fr".into()),
            title: Some("Un titre de page correct".into()),
            meta_robots: vec!["noindex".into()],
            ..Default::default()
        }
    }

    #[test]
    fn daily_schedule_picks_next_occurrence_strictly_after_now() {
        let s = Schedule::Daily { hour: 2, minute: 0 };
        let before = Local.with_ymd_and_hms(2026, 9, 20, 1, 30, 0).unwrap();
        let at = s.next_run(before);
        assert_eq!((at.day(), at.hour(), at.minute()), (20, 2, 0));

        let exactly = Local.with_ymd_and_hms(2026, 9, 20, 2, 0, 0).unwrap();
        assert_eq!(
            s.next_run(exactly).day(),
            21,
            "same instant rolls to tomorrow"
        );

        let after = Local.with_ymd_and_hms(2026, 9, 20, 23, 59, 0).unwrap();
        assert_eq!(s.next_run(after).day(), 21);
    }

    #[test]
    fn interval_schedule_adds_interval() {
        let now = Local::now();
        let at = Schedule::Every(Duration::from_secs(3600)).next_run(now);
        assert_eq!(at - now, chrono::Duration::hours(1));
    }

    #[test]
    fn job_validation_rejects_bad_inputs() {
        assert!(nightly_job().validate().is_ok());
        assert!(ScheduledJob::nightly("x", vec![]).validate().is_err());
        assert!(ScheduledJob::nightly(" ", vec![URL.into()])
            .validate()
            .is_err());
        assert!(nightly_job()
            .with_schedule(Schedule::Daily {
                hour: 24,
                minute: 0
            })
            .validate()
            .is_err());
        assert!(nightly_job()
            .with_schedule(Schedule::Every(Duration::from_secs(5)))
            .validate()
            .is_err());
    }

    #[test]
    fn run_without_template_triages_everything_and_proposes_nothing() {
        let run = plan_run(
            &nightly_job(),
            vec![audit(
                noindex_page(),
                vec![rgaa_result("1.1", CriterionStatus::Fail, "critical")],
            )],
            None,
            Local::now(),
        );
        assert!(!run.findings.is_empty());
        assert_eq!(run.lifecycles.len(), run.findings.len());
        assert_eq!(run.count_in(FindingState::Triaged), run.findings.len());
        assert!(run.proposals.is_empty());
        assert_eq!(run.findings[0].criticality.as_str(), "P0");
        assert_eq!(
            run.findings[0].origins,
            vec!["rgaa"],
            "RGAA P0 outranks SEO P0"
        );
        assert_eq!(run.findings[1].finding.rule, "SEO-META-05");
        for lc in &run.lifecycles {
            assert_eq!(lc.history()[0].actor, SCHEDULER_ACTOR);
            assert!(lc.history()[0].reason.contains("nightly-seo"));
        }
    }

    #[test]
    fn run_with_template_queues_patches_awaiting_approval_and_flags_content_for_review() {
        let head = "<html lang=\"fr\"><head><meta name=\"robots\" content=\"noindex\"><title>Un titre de page correct</title></head><body></body></html>";
        let job = nightly_job().with_head_template("src/layout.html");
        let run = plan_run(
            &job,
            vec![audit(noindex_page(), vec![])],
            Some(head),
            Local::now(),
        );

        let state_of = |rule: &str| {
            let idx = run
                .findings
                .iter()
                .position(|m| m.finding.rule == rule)
                .unwrap();
            run.lifecycles[idx].state
        };
        // Deterministic <head> fixes are proposed and parked behind approval.
        assert_eq!(state_of("SEO-META-05"), FindingState::AwaitingApproval);
        assert_eq!(state_of("SEO-CANON-01"), FindingState::AwaitingApproval);
        // Content rules are routed to human/LLM review, never invented.
        assert_eq!(state_of("SEO-META-03"), FindingState::NeedsReview);
        assert_eq!(state_of("SEO-HEAD-01"), FindingState::NeedsReview);

        for outcome in &run.proposals {
            if let RemediationOutcome::Ok(g) = outcome {
                assert!(g.proposal.requires_approval());
                assert!(g.proposal.ensure_approved().is_err());
                assert_eq!(g.proposal.files, vec!["src/layout.html"]);
            }
        }
        assert_eq!(run.count_in(FindingState::AwaitingApproval), 2);
        assert!(
            run.count_in(FindingState::Applied) == 0 && run.count_in(FindingState::Resolved) == 0
        );
    }

    #[test]
    fn approval_is_forced_even_if_job_policy_disables_it() {
        let head = "<html><head><meta name=\"robots\" content=\"noindex\"></head></html>";
        let mut job = nightly_job().with_head_template("layout.html");
        job.policy.require_approval = false;
        let run = plan_run(
            &job,
            vec![audit(noindex_page(), vec![])],
            Some(head),
            Local::now(),
        );
        let ok = run.proposals.iter().filter_map(|o| match o {
            RemediationOutcome::Ok(g) => Some(g),
            _ => None,
        });
        let mut any = false;
        for g in ok {
            any = true;
            assert!(g.proposal.requires_approval());
        }
        assert!(any, "expected at least one proposal");
    }

    #[test]
    fn summary_json_counts_states() {
        let run = plan_run(
            &nightly_job(),
            vec![audit(noindex_page(), vec![])],
            None,
            Local::now(),
        );
        let s = run.summary_json();
        assert_eq!(s["job"], "nightly-seo");
        assert_eq!(s["audits"], 1);
        assert_eq!(s["triaged"], run.findings.len());
        assert_eq!(s["awaiting_approval"], 0);
        assert!(s["by_criticality"]["P0"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn run_until_returns_promptly_when_stopped() {
        let scheduler = Scheduler::new(Arc::new(Orchestrator::new()));
        let (tx, rx) = tokio::sync::watch::channel(false);
        let job = nightly_job().with_schedule(Schedule::Every(Duration::from_secs(3600)));
        let handle = tokio::spawn(async move { scheduler.run_until(vec![job], rx).await });
        tx.send(true).unwrap();
        let result = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("loop exits after stop")
            .expect("task did not panic");
        assert!(result.is_ok());
    }
}
