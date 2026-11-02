//! Exercise every enabled product compression policy through the native SDK.
use super::*;
use crate::application::short_term_memory::ContextProvidersConfig;
use crate::domain::agent::{AgentMessage, AgentMessagePart};
use eliteforge_agent_framework as sdk;
use fw::ContextReducer;
use serde_json::json;

struct SummaryModel;
#[async_trait]
impl fw::ChatModel for SummaryModel {
    async fn generate(
        &self,
        _: &str,
        request: fw::ChatRequest,
        _: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        assert!(request.tools.is_empty());
        Ok(fw::ChatResponse::text(
            json!({"summary":"retained historical facts"}).to_string(),
        ))
    }
}
struct CompressionMatrix(fw::ContextCompressionTrigger);
#[async_trait]
impl fw::ModelWorkflow for CompressionMatrix {
    async fn run(
        &self,
        run: &fw::RunContext,
        models: &dyn fw::ChatExecutor,
        limits: &fw::RunLimits,
        model: &fw::ModelRef,
    ) -> Result<fw::ChatResponse, fw::AgentError> {
        let config = ContextProvidersConfig::default();
        let estimator = fw::ConservativeTokenEstimator;
        let history = fw::ConversationSnapshot::new(
            fw::ConversationKey::new("matrix"),
            "v1",
            vec![
                fw::Message::user("old question ".repeat(400)),
                fw::Message::assistant("old answer ".repeat(400)),
                fw::Message::user("latest question"),
            ],
        );
        let budget = fw::ContextBudget {
            max_tokens: 512,
            max_messages: 16,
        };
        let compression = Some(fw::ContextCompressionDirective {
            trigger: self.0,
            target_tokens: 400,
        });
        let mut tested = Vec::new();
        for (id, policy) in &config.data_blocks {
            if policy.policy.strategy != Strategy::Bound {
                continue;
            }
            {
                let fitting = fw::ContextBlock {
                    id: id.clone(), trust: fw::ContextTrust::Data, source_version: None,
                    messages: vec![messages::encode(&AgentMessage::new("user", vec![
                        AgentMessagePart::Context { kind: id.clone(), text: "current reference".into(), metadata: Some(json!({"label":"current reference"})) },
                    ])).unwrap()],
                };
                let kept = bound_data_reducer(policy.max_tokens.unwrap() as usize)
                    .reduce(&fw::ContextReductionInput {
                        block: &fitting, budget, model, estimator: &estimator,
                        history: &history, run, models, limits,
                        overflow_recoveries: 0, compression: None,
                    }).await?;
                assert_eq!(kept, fitting.messages, "fitting data must not be reformatted or marked reduced: {id}");
            }
            let mut block = fw::ContextBlock {
                id: id.clone(),
                trust: fw::ContextTrust::Data,
                source_version: None,
                messages: vec![
                    messages::encode(&AgentMessage::new(
                        "user",
                        vec![AgentMessagePart::Context {
                            kind: id.clone(),
                            text: format!("FACT-{id} {}", "context details ".repeat(500)),
                            metadata: None,
                        }],
                    ))
                    .unwrap(),
                ],
            };
            if matches!(id.as_str(), "application_context" | "knowledge_base") {
                block.messages[0].content.0.push(fw::ContentBlock::Media {
                    media: fw::MediaContent {
                        mime_type: "image/png".into(),
                        source: fw::MediaSource::Base64("MEDIA_BYTES_SENTINEL".into()),
                        name: Some("figure".into()),
                        estimated_tokens: Some(2048),
                    },
                });
            }
            let original = block.clone();
            let reduced = bound_data_reducer(policy.max_tokens.unwrap() as usize)
                .reduce(&fw::ContextReductionInput {
                    block: &block,
                    budget,
                    model,
                    estimator: &estimator,
                    history: &history,
                    run,
                    models,
                    limits,
                    overflow_recoveries: 0,
                    compression,
                })
                .await?;
            verify(id, &block, &reduced, budget.max_tokens, model, &estimator).await;
            assert_eq!(block.messages, original.messages, "source changed: {id}");
            if matches!(id.as_str(), "application_context" | "knowledge_base") {
                assert!(!reduced[0].content.has_media());
                assert!(reduced[0].content.text().contains("not shown to the model"));
                assert!(!reduced[0].content.text().contains("MEDIA_BYTES_SENTINEL"));
            }
            tested.push(id.clone());
        }
        for (id, part, policy) in [
            (
                "conversation_history_summary",
                AgentMessagePart::CompactionCheckpoint {
                    summary: "checkpoint summary ".repeat(300),
                    recent: "recent context ".repeat(300),
                    source_start_seq: 1,
                    source_end_seq: 10,
                },
                &config.checkpoint,
            ),
            (
                "current_session_working_state",
                AgentMessagePart::Context {
                    kind: "current_session_working_state".into(),
                    text: "active todo ".repeat(500),
                    metadata: None,
                },
                &config.working,
            ),
        ] {
            assert_eq!(policy.strategy, Strategy::Bound);
            let block = fw::ContextBlock {
                id: id.into(),
                trust: fw::ContextTrust::Data,
                source_version: None,
                messages: vec![messages::encode(&AgentMessage::new("user", vec![part])).unwrap()],
            };
            let reduced = bound_memory_reducer()
                .reduce(&fw::ContextReductionInput {
                    block: &block,
                    budget,
                    model,
                    estimator: &estimator,
                    history: &history,
                    run,
                    models,
                    limits,
                    overflow_recoveries: 0,
                    compression,
                })
                .await?;
            verify(id, &block, &reduced, budget.max_tokens, model, &estimator).await;
            tested.push(id.into());
        }
        assert_eq!(config.history.strategy, Strategy::Checkpoint);
        let block = fw::ContextBlock {
            id: fw::HISTORY_BLOCK.into(),
            trust: fw::ContextTrust::Data,
            source_version: None,
            messages: history.messages.clone(),
        };
        let compressor = memory::ModelCompressor::new(
            model.clone(),
            memory::CompressionOptions {
                max_input_tokens: 16000,
                max_summary_bytes: 256,
                max_model_calls: 16,
            },
        )?;
        let reduced = compressor
            .reduce(&fw::ContextReductionInput {
                block: &block,
                budget,
                model,
                estimator: &estimator,
                history: &history,
                run,
                models,
                limits,
                overflow_recoveries: 0,
                compression,
            })
            .await?;
        verify(
            fw::HISTORY_BLOCK,
            &block,
            &reduced,
            budget.max_tokens,
            model,
            &estimator,
        )
        .await;
        assert_eq!(reduced.last().unwrap().content.text(), "latest question");
        assert!(
            reduced
                .iter()
                .any(|m| m.content.text().contains("retained historical facts"))
        );
        tested.push(fw::HISTORY_BLOCK.into());
        for id in ["runtime_identity", "skills.catalog", "tool_domains", "agent_directory"] {
            assert_eq!(config.data_blocks[id].policy.strategy, Strategy::Keep);
            let block = fw::ContextBlock::data(id, "preserved data ".repeat(500));
            let input = fw::ContextReductionInput {
                block: &block,
                budget,
                model,
                estimator: &estimator,
                history: &history,
                run,
                models,
                limits,
                overflow_recoveries: 0,
                compression,
            };
            assert!(matches!(
                fw::KeepContext.reduce(&input).await.unwrap_err().primary(),
                fw::AgentError::Context(fw::ContextError::BudgetExceeded(_))
            ));
            let kept = fw::KeepContext
                .reduce(&fw::ContextReductionInput {
                    budget: fw::ContextBudget {
                        max_tokens: 16000,
                        ..budget
                    },
                    ..input
                })
                .await?;
            assert_eq!(kept, block.messages);
        }
        assert_eq!(
            tested.len(),
            9,
            "all configured compression blocks must be covered"
        );
        Ok(fw::ChatResponse::text(tested.join(",")))
    }
}
async fn verify(
    id: &str,
    source: &fw::ContextBlock,
    reduced: &[fw::Message],
    cap: usize,
    model: &fw::ModelRef,
    estimator: &dyn fw::TokenEstimator,
) {
    assert!(!reduced.is_empty(), "empty projection: {id}");
    let mut before = 0;
    let mut after = 0;
    for m in &source.messages {
        before += estimator.message(model, m).await.unwrap();
    }
    for m in reduced {
        after += estimator.message(model, m).await.unwrap();
    }
    assert!(
        after <= cap && after < before,
        "{id}: {before} -> {after} / {cap}"
    );
    eprintln!("compression verified {id}: {before} -> {after} tokens / {cap}");
    let request = fw::ChatRequest {
        messages: reduced.to_vec(),
        tools: vec![],
        options: Default::default(),
        extensions: Default::default(),
    };
    let wire =
        crate::adapters::outbound::model::http_model_client::native_request(&request).unwrap();
    let text = wire
        .messages
        .iter()
        .map(|m| m.plain_text_content())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !text.contains("tronMessagePart"),
        "host record leaked in {id}"
    );
    assert!(
        !text.contains("wireMessagePart"),
        "wire record leaked in {id}"
    );
    assert!(
        reduced.iter().all(|m| m.role == fw::Role::User),
        "data promoted to instructions: {id}"
    );
}

#[tokio::test]
async fn every_configured_compression_block_is_bounded_without_host_record_leaks() {
    let client = sdk::ModelClient::new(sdk::ModelRuntimeUpdate::new(
        "matrix",
        1,
        vec![fw::ModelConnection::new(
            "matrix",
            Arc::new(SummaryModel),
            vec![fw::ModelDescriptor::new("summary")],
        )],
    ))
    .unwrap();
    for trigger in [
        fw::ContextCompressionTrigger::Block,
        fw::ContextCompressionTrigger::Global,
    ] {
        client
            .workflow_observed(
                &fw::ModelRef::new("matrix", "summary"),
                Arc::new(CompressionMatrix(trigger)),
                &fw::RunLimits {
                    max_context_tokens: 32000,
                    max_model_output_tokens: 512,
                    max_model_calls_per_step: Some(16),
                    ..Default::default()
                },
                "compression-matrix",
            )
            .await
            .into_result()
            .unwrap();
    }
    client.close().await;
}

struct ExactEstimator;
#[async_trait]
impl fw::TokenEstimator for ExactEstimator {
    async fn message(&self, _: &fw::ModelRef, message: &fw::Message) -> Result<usize, fw::ModelError> { Ok(message.content.text().len()) }
    async fn tools(&self, _: &fw::ModelRef, _: &[fw::ToolDescriptor]) -> Result<usize, fw::ModelError> { Ok(0) }
    async fn options(&self, _: &fw::ModelRef, _: &fw::GenerationOptions) -> Result<usize, fw::ModelError> { Ok(0) }
}
struct ThresholdSource(&'static str, usize);
#[async_trait]
impl fw::ContextContributor for ThresholdSource {
    fn block_id(&self) -> &str { self.0 }
    async fn contribute(&self, _: &fw::ContextInput<'_>) -> Result<fw::ContextBlock, fw::AgentError> { Ok(fw::ContextBlock::data(self.0, "x".repeat(self.1))) }
}
struct Shorten;
#[async_trait]
impl fw::ContextReducer for Shorten {
    async fn reduce(&self, input: &fw::ContextReductionInput<'_>) -> Result<Vec<fw::Message>, fw::AgentError> {
        assert!(input.compression.is_some());
        Ok(vec![fw::Message::user("kept")])
    }
}
struct ThresholdMatrix(u8);
struct SlowShorten;
#[async_trait]
impl fw::ContextReducer for SlowShorten {
    async fn reduce(&self, input: &fw::ContextReductionInput<'_>) -> Result<Vec<fw::Message>, fw::AgentError> {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Shorten.reduce(input).await
    }
}
#[async_trait]
impl fw::ModelWorkflow for ThresholdMatrix {
    async fn run(&self, run: &fw::RunContext, models: &dyn fw::ChatExecutor, limits: &fw::RunLimits, model: &fw::ModelRef) -> Result<fw::ChatResponse, fw::AgentError> {
        let mut context = run.clone();
        if self.0 == 2 { context.extensions.insert(fw::ContextCompactionRequest); }
        let history = fw::ConversationSnapshot::new(fw::ConversationKey::new("threshold"), "g1", vec![fw::Message::user("question")]);
        let mut layout = vec![fw::ContextBlockConfig::new(fw::INSTRUCTIONS_BLOCK, fw::BlockBudget::tokens(100)),
            fw::ContextBlockConfig::new("a", fw::BlockBudget::tokens(900)).compress_at(10_000, 8_000).reduce_with(Shorten)];
        if self.0 == 4 { layout[1] = layout[1].clone().reduce_with(SlowShorten); }
        let mut sources: Vec<Arc<dyn fw::ContextContributor>> = vec![Arc::new(ThresholdSource("a", match self.0 { 1 | 4 => 950, 3 => 600, _ => 150 }))];
        if self.0 == 3 {
            layout.push(fw::ContextBlockConfig::new("b", fw::BlockBudget::tokens(900)).compress_at(10_000, 8_000).reduce_with(Shorten));
            sources.push(Arc::new(ThresholdSource("b",600)));
        }
        layout.push(fw::ContextBlockConfig::new(fw::HISTORY_BLOCK, fw::BlockBudget::tokens(1000)));
        let mut assembler=fw::DefaultContextAssembler::new(fw::ContextLayout(layout),sources)?
            .with_policy(Arc::new(memory::BorrowUnusedMemoryBudget))
            .with_global_compression(match self.0 { 0 => Some(2000), 3 => Some(8000), _ => None },false)?;
        if self.0 == 4 { assembler.source_timeout=std::time::Duration::from_millis(1); }
        let window=assembler.assemble(&fw::ContextAssemblyInput { stage:"threshold",attempt:1,step:1,model,
            budget:fw::ContextBudget {max_tokens:1000,max_messages:64},estimator:&ExactEstimator,
            history:&history,instructions:&[fw::Message::system("i".repeat(100))],tools:&[],options:&Default::default(),
            run:&context,models,limits,overflow_recoveries:0 }).await?;
        assert!(window.report.estimated_input_tokens<=1000);
        assert_eq!(window.report.blocks[0].compression_trigger,fw::ContextCompressionTrigger::None);
        assert_eq!(window.report.blocks[0].final_tokens,100);
        for block in window.report.blocks.iter().filter(|b|b.id=="a"||b.id=="b") {
            assert_eq!(block.compression_trigger,if matches!(self.0,1|4) {fw::ContextCompressionTrigger::Block} else {fw::ContextCompressionTrigger::Global});
            assert_eq!(block.final_tokens,4);
        }
        Ok(fw::ChatResponse::text("verified"))
    }
}
#[tokio::test]
async fn compaction_triggers_for_total_usage_individual_blocks_manual_and_over_capacity() {
    let client=sdk::ModelClient::new(sdk::ModelRuntimeUpdate::new("threshold",1,vec![fw::ModelConnection::new("matrix",Arc::new(SummaryModel),vec![fw::ModelDescriptor::new("summary")])])).unwrap();
    for mode in 0..5 {
        client.workflow_observed(&fw::ModelRef::new("matrix","summary"),Arc::new(ThresholdMatrix(mode)),&fw::RunLimits::default(),"threshold-regression").await.into_result().unwrap();
    }
    client.close().await;
}
