use super::*;
use super::{
    candidate::{CandidateIndex, ExperimentRunner, ReusedVectors, pinned_settings, ranges},
    runtime::{BudgetState, QualityRuntime},
    snapshot::QualitySnapshot,
};
use crate::application::document::{ChunkConfig, ChunkMode};
use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

impl KnowledgeQualityService {
    pub async fn start(
        &self,
        user: &str,
        kb: &str,
        request_id: &str,
        options: QualityOptimizationOptions,
    ) -> Result<QualityOptimizationRun, KnowledgeBaseError> {
        self.host.owned(user, kb)?;
        options.validate()?;
        if options.candidates.iter().any(|c| {
            !self
                .host
                .chunk_strategies
                .descriptors()
                .iter()
                .any(|d| d.id == c.mode.strategy_id())
        }) {
            return Err(invalid("候选切片策略未注册"));
        }
        if request_id.is_empty() || request_id.len() > 120 {
            return Err(invalid("需要有效的优化请求标识"));
        }
        let snapshot = self.snapshot(user, kb).await?;
        let now = chrono::Utc::now().timestamp_millis();
        let run = QualityOptimizationRun {
            id: eliteforge_agent_core::SessionRecord::new("").id,
            library_id: kb.into(),
            owner_user_code: user.into(),
            request_id: request_id.into(),
            source_fingerprint: snapshot.fingerprint.clone(),
            dataset_revision: self
                .dataset(user, kb)?
                .map_or(0, |dataset| dataset.revision),
            cases: vec![],
            status: "QUEUED".into(),
            stage: "DATASET".into(),
            options,
            baseline_training: None,
            baseline_holdout: None,
            trials: vec![],
            selected_trial: None,
            published: false,
            target_reached: false,
            missing: vec![],
            model_operations: 0,
            estimated_input_tokens: 0,
            warnings: vec![],
            error: None,
            created_at: now,
            updated_at: now,
        };
        let stored = self.host.repository.insert_quality_run(&run)?;
        if stored.id != run.id {
            return Ok(stored);
        }
        let token = CancellationToken::new();
        self.host
            .quality_jobs
            .jobs
            .lock()
            .map_err(invalid)?
            .insert(run.id.clone(), token.clone());
        let service = self.clone();
        let owner = user.to_owned();
        let budget = BudgetState::new(run.options.budget.clone(), token);
        crate::shared::auth_holder::AuthHolder::spawn(async move {
            let mut run = run;
            let result = tokio::time::timeout(
                Duration::from_secs(run.options.budget.max_duration_seconds),
                service.optimize(&owner, &snapshot, &mut run, &budget),
            )
            .await;
            let error = match result {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(error.to_string()),
                Err(_) => Some("优化达到时间预算，未发布候选索引".into()),
            };
            if let Some(error) = error {
                if let Err(cleanup) = service
                    .host
                    .repository
                    .discard_quality_staging(&run.library_id, &run.id)
                {
                    tracing::error!("quality candidate cleanup failed: {cleanup}");
                }
                run.published = false;
                run.error = Some(error.clone());
                run.stage = "COMPLETED".into();
                run.status = if budget.token.is_cancelled()
                    || service
                        .host
                        .repository
                        .quality_run(&owner, &run.library_id, &run.id)
                        .is_ok_and(|stored| stored.status == "CANCEL_REQUESTED")
                {
                    "CANCELLED"
                } else if error.contains("预算") {
                    "BUDGET_LIMIT"
                } else if matches!(error.as_str(), "文档正在索引或状态已改变，请刷新后重试")
                {
                    "STALE"
                } else {
                    "FAILED"
                }
                .into();
                update_usage(&mut run, &budget);
                let _ = service.host.repository.save_quality_run(&run);
            }
            if let Ok(mut jobs) = service.host.quality_jobs.jobs.lock() {
                jobs.remove(&run.id);
            }
        });
        Ok(stored)
    }
    pub fn run(
        &self,
        user: &str,
        kb: &str,
        id: &str,
    ) -> Result<QualityOptimizationRun, KnowledgeBaseError> {
        self.host.owned(user, kb)?;
        self.host.repository.quality_run(user, kb, id)
    }
    pub fn runs(
        &self,
        user: &str,
        kb: &str,
    ) -> Result<Vec<QualityOptimizationRun>, KnowledgeBaseError> {
        self.host.owned(user, kb)?;
        self.host.repository.quality_runs(user, kb)
    }
    pub fn cancel(
        &self,
        user: &str,
        kb: &str,
        id: &str,
    ) -> Result<QualityOptimizationRun, KnowledgeBaseError> {
        self.host.owned(user, kb)?;
        let run = self.host.repository.cancel_quality_run(user, kb, id)?;
        if let Some(token) = self.host.quality_jobs.jobs.lock().map_err(invalid)?.get(id) {
            token.cancel();
        }
        Ok(run)
    }
    async fn optimize(
        &self,
        user: &str,
        snapshot: &QualitySnapshot,
        run: &mut QualityOptimizationRun,
        budget: &Arc<BudgetState>,
    ) -> Result<(), KnowledgeBaseError> {
        budget.check().map_err(invalid)?;
        run.status = "RUNNING".into();
        self.host.repository.save_quality_run(run)?;
        let dataset = self
            .ensure_dataset(user, snapshot, run.options.max_cases, budget)
            .await?;
        run.dataset_revision = dataset.revision;
        run.cases = dataset.cases.clone();
        if dataset
            .cases
            .iter()
            .any(|case| case.provenance == EvaluationProvenance::Generated)
        {
            run.warnings
                .push("包含自动生成题，结果属于原文自测，不代表经人工标注的业务召回率".into());
        }
        run.stage = "BASELINE".into();
        update_usage(run, budget);
        self.host.repository.save_quality_run(run)?;
        let settings = pinned_settings(snapshot)?;
        let baseline = ExperimentRunner {
            service: self.host.rag.with_runtime(QualityRuntime {
                inner: self.host.rag.runtime_handle(),
                budget: budget.clone(),
            }),
            host: Some(&self.host),
            user,
            snapshot,
            ranges: ranges(
                snapshot,
                &settings,
                snapshot
                    .documents
                    .iter()
                    .filter_map(|doc| doc.document.active_source_code.clone())
                    .collect(),
                None,
            ),
            contexts: None,
            rerank: settings.rerank_model_code.clone(),
            budget: budget.clone(),
        };
        let (training, _) = evaluate_retrieval(
            &baseline,
            &dataset.cases,
            EvaluationPartition::Training,
            run.options.top_k,
        )
        .await
        .map_err(invalid)?;
        let (holdout, missing) = evaluate_retrieval(
            &baseline,
            &dataset.cases,
            EvaluationPartition::Holdout,
            run.options.top_k,
        )
        .await
        .map_err(invalid)?;
        self.validate_snapshot(user, snapshot)?;
        run.baseline_training = Some(training.clone());
        run.baseline_holdout = Some(holdout.clone());
        run.missing = missing
            .into_iter()
            .filter(|score| !score.missing.is_empty())
            .collect();
        if holdout.recall >= run.options.target_recall {
            run.target_reached = true;
            run.status = "COMPLETED".into();
            run.stage = "COMPLETED".into();
            update_usage(run, budget);
            self.host.repository.save_quality_run(run)?;
            return Ok(());
        }
        let candidates = if run.options.candidates.is_empty() {
            default_candidates(
                &snapshot.library.default_chunk_config,
                run.options.max_trials,
            )
        } else {
            run.options.candidates.clone()
        };
        let mut reuse = ReusedVectors::default();
        let mut best: Option<(
            usize,
            CandidateIndex,
            ChunkConfig,
            RetrievalEvaluationMetrics,
        )> = None;
        for (number, config) in candidates.into_iter().enumerate() {
            budget.check().map_err(invalid)?;
            self.validate_snapshot(user, snapshot)?;
            run.stage = format!("TRIAL_{}", number + 1);
            update_usage(run, budget);
            self.host.repository.save_quality_run(run)?;
            let started = Instant::now();
            let descriptor = self
                .host
                .chunk_strategies
                .descriptors()
                .into_iter()
                .find(|descriptor| descriptor.id == config.mode.strategy_id())
                .ok_or_else(|| invalid("候选切片策略未注册"))?;
            let candidate = match self
                .build_candidate(user, run, snapshot, &config, number, budget, &mut reuse)
                .await
            {
                Ok(candidate) => candidate,
                Err(error) if error.to_string().starts_with("切片策略失败：") => {
                    run.trials.push(QualityTrial {
                        strategy: descriptor,
                        chunk_config: config,
                        training: None,
                        holdout: None,
                        chunk_count: 0,
                        duration_ms: started.elapsed().as_millis() as u64,
                        error: Some(error.to_string()),
                    });
                    continue;
                }
                Err(error) => return Err(error),
            };
            run.warnings.extend(
                candidate
                    .warnings
                    .iter()
                    .map(|warning| format!("Candidate {}: {warning}", number + 1)),
            );
            let runner = ExperimentRunner {
                service: candidate.rag.clone(),
                host: None,
                user,
                snapshot,
                ranges: candidate.ranges.clone(),
                contexts: Some(&candidate.contexts),
                rerank: candidate.settings.rerank_model_code.clone(),
                budget: budget.clone(),
            };
            let (score, _) = evaluate_retrieval(
                &runner,
                &dataset.cases,
                EvaluationPartition::Training,
                run.options.top_k,
            )
            .await
            .map_err(invalid)?;
            let index = run.trials.len();
            let count = candidate.chunk_count;
            run.trials.push(QualityTrial {
                strategy: descriptor,
                chunk_config: config.clone(),
                training: Some(score.clone()),
                holdout: None,
                chunk_count: count,
                duration_ms: started.elapsed().as_millis() as u64,
                error: None,
            });
            let improves_training = score.recall + 1e-9 >= training.recall;
            let wins = best.as_ref().is_none_or(|(_, old, _, old_score)| {
                score.recall > old_score.recall + 1e-9
                    || ((score.recall - old_score.recall).abs() < 1e-9
                        && (score.mrr > old_score.mrr + 1e-9
                            || ((score.mrr - old_score.mrr).abs() < 1e-9
                                && count < old.chunk_count)))
            });
            if improves_training && wins {
                best = Some((index, candidate, config, score));
            }
            update_usage(run, budget);
            self.host.repository.save_quality_run(run)?;
        }
        if let Some((index, candidate, config, _)) = best {
            run.selected_trial = Some(index);
            run.stage = "HOLDOUT".into();
            update_usage(run, budget);
            self.host.repository.save_quality_run(run)?;
            let runner = ExperimentRunner {
                service: candidate.rag.clone(),
                host: None,
                user,
                snapshot,
                ranges: candidate.ranges.clone(),
                contexts: Some(&candidate.contexts),
                rerank: candidate.settings.rerank_model_code.clone(),
                budget: budget.clone(),
            };
            // Only the training winner sees holdout. Do not pick repeatedly
            // against validation answers until a flattering score appears.
            let (validation, scores) = evaluate_retrieval(
                &runner,
                &dataset.cases,
                EvaluationPartition::Holdout,
                run.options.top_k,
            )
            .await
            .map_err(invalid)?;
            run.trials[index].holdout = Some(validation.clone());
            run.missing = scores
                .into_iter()
                .filter(|score| !score.missing.is_empty())
                .collect();
            if can_publish(&holdout, &validation, run.options.minimum_improvement) {
                budget.check().map_err(invalid)?;
                self.validate_snapshot(user, snapshot)?;
                run.status = "PUBLISHING".into();
                run.stage = "PUBLISHING".into();
                run.target_reached = validation.recall >= run.options.target_recall;
                update_usage(run, budget);
                self.host.repository.save_quality_run(run)?;
                self.publish_candidate(user, snapshot, run, &config, candidate)
                    .await?;
                return Ok(());
            }
            run.warnings
                .push("独立验证题未确认改善，当前索引未替换".into());
        } else {
            run.warnings
                .push("没有候选方案达到当前训练题表现，当前索引未替换".into());
        }
        run.status = "COMPLETED".into();
        run.stage = "COMPLETED".into();
        update_usage(run, budget);
        self.host.repository.save_quality_run(run)?;
        Ok(())
    }
}

fn update_usage(run: &mut QualityOptimizationRun, budget: &BudgetState) {
    run.updated_at = chrono::Utc::now().timestamp_millis();
    run.model_operations = budget.operations.load(Ordering::SeqCst);
    run.estimated_input_tokens = budget.input_tokens.load(Ordering::SeqCst);
}
pub(crate) fn can_publish(
    baseline: &RetrievalEvaluationMetrics,
    candidate: &RetrievalEvaluationMetrics,
    minimum: f64,
) -> bool {
    candidate.cases >= 4
        && candidate.recall - baseline.recall > 1e-9
        && candidate.recall - baseline.recall + 1e-9 >= minimum
        && candidate.mrr + 0.05 >= baseline.mrr
}
fn default_candidates(defaults: &ChunkConfig, limit: usize) -> Vec<ChunkConfig> {
    let sizes = [
        defaults.max_chars.max(800),
        (defaults.max_chars * 2).min(8000),
        defaults.max_chars.max(1600),
        defaults.max_chars,
        600,
        2400,
    ];
    [
        ChunkMode::Auto,
        ChunkMode::Structure,
        ChunkMode::Heading,
        ChunkMode::Fixed,
        ChunkMode::Auto,
        ChunkMode::Structure,
    ]
    .into_iter()
    .zip(sizes)
    .take(limit)
    .map(|(mode, max_chars)| ChunkConfig {
        mode,
        max_chars,
        overlap_chars: defaults.overlap_chars.min(max_chars / 10),
        heading_depth: 3,
    })
    .collect()
}
