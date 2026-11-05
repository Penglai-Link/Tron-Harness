pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}

pub const TOOL_POLICY_SAVE_FAILED: ApplicationErrorDescriptor = ApplicationErrorDescriptor {
    code: "101038001",
    message_key: "tron.capability.tool_policy_save_failed",
};

pub const TOOL_POLICY_INVALID: ApplicationErrorDescriptor = ApplicationErrorDescriptor {
    code: "101038002",
    message_key: "tron.capability.tool_policy_invalid",
};
