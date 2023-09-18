use revolt_models::v0::*;

impl From<BotInformation> for crate::BotInformation {
    fn from(value: BotInformation) -> Self {
        crate::BotInformation {
            owner: value.owner_id,
            model: value.model.map(|x| x.into()),
            welcome: value.welcome,
        }
    }
}

impl From<BotType> for crate::BotType {
    fn from(value: BotType) -> Self {
        match value {
            BotType::CustomBot => crate::BotType::CustomBot,
            BotType::PromptBot => crate::BotType::PromptBot,
        }
    }
}

impl From<BotModel> for crate::BotModel {
    fn from(value: BotModel) -> Self {
        crate::BotModel {
            model_name: value.model_name,
            prompts: value.prompts.into(),
            temperature: value.temperature,
        }
    }
}

impl From<PromptTemplate> for crate::PromptTemplate {
    fn from(value: PromptTemplate) -> Self {
        crate::PromptTemplate {
            system_prompt: value.system_prompt,
        }
    }
}

impl From<Component> for crate::Component {
    fn from(value: Component) -> Self {
        match value {
            Component::Button {
                label,
                style,
                enabled,
            } => crate::Component::Button {
                label,
                style,
                enabled,
            },
            Component::LineBreak => crate::Component::LineBreak,
            Component::Status { label } => crate::Component::Status { label },
        }
    }
}
