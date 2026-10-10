use super::*;

pub(super) type ProviderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ModelProviderError>> + Send + 'a>>;

pub(super) fn provider_quality_from_sdk(quality: QualityType) -> ProviderModelQuality {
    match quality {
        QualityType::High => ProviderModelQuality::High,
        QualityType::Middle => ProviderModelQuality::Middle,
        QualityType::Low => ProviderModelQuality::Low,
    }
}

pub(super) fn provider_quality_to_sdk(quality: ProviderModelQuality) -> QualityType {
    match quality {
        ProviderModelQuality::High => QualityType::High,
        ProviderModelQuality::Middle => QualityType::Middle,
        ProviderModelQuality::Low => QualityType::Low,
    }
}

pub(super) fn agent_protocol_from_sdk(protocol: ProtocolType) -> AgentModelProtocol {
    match protocol {
        ProtocolType::ChatCompletions => AgentModelProtocol::ChatCompletions,
        ProtocolType::Responses => AgentModelProtocol::Responses,
    }
}
