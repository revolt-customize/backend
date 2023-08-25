use serde::{Deserialize, Serialize};
use validator::Validate;

/// Model information for prompt bot
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Validate, PartialEq)]
pub struct BotModel {
    pub model_name: String,
    pub prompts: PromptTemplate,
    #[validate(range(min = 0.0, max = 1.0))]
    pub temperature: f32,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
pub struct PromptTemplate {
    pub system_prompt: String,
}

impl Default for BotModel {
    fn default() -> Self {
        Self {
            model_name: "gpt-3.5-turbo".to_owned(),
            prompts: Default::default(),
            temperature: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use validator::Validate;

    use crate::models::{
        prompt::{BotModel, PromptTemplate},
        user::BotInformation,
    };

    #[test]
    fn test_default_bot() {
        let bot_information = BotInformation {
            owner: "id1".into(),
            model: Some(Default::default()),
        };

        assert_eq!(
            bot_information.model.clone().unwrap(),
            BotModel {
                model_name: "gpt-3.5-turbo".into(),
                prompts: PromptTemplate {
                    system_prompt: "".into()
                },
                temperature: 0.0,
            }
        );
    }

    #[test]
    fn test_validate() {
        let bot_model = BotModel {
            temperature: 1.4,
            ..Default::default()
        };

        assert!(bot_model.validate().is_err());
    }
}
