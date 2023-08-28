use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

/// Utility function to check if a boolean value is false
pub fn if_false(t: &bool) -> bool {
    !t
}

/// Bot flag enum
#[derive(Debug, PartialEq, Eq, TryFromPrimitive, Copy, Clone)]
#[repr(i32)]
pub enum BotFlags {
    Verified = 1,
    Official = 2,
}

/// Representation of a bot on Revolt
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, OptionalStruct, Default)]
#[optional_derive(Serialize, Deserialize, JsonSchema, Debug, Default, Clone)]
#[optional_name = "PartialBot"]
#[opt_skip_serializing_none]
#[opt_some_priority]
pub struct Bot {
    /// Bot Id
    ///
    /// This equals the associated bot user's id.
    #[serde(rename = "_id")]
    pub id: String,
    /// User Id of the bot owner
    pub owner: String,
    /// Token used to authenticate requests for this bot
    pub token: String,
    /// Whether the bot is public
    /// (may be invited by anyone)
    pub public: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_type: Option<BotType>,
    /// Whether to enable analytics
    #[serde(skip_serializing_if = "if_false", default)]
    pub analytics: bool,
    /// Whether this bot should be publicly discoverable
    #[serde(skip_serializing_if = "if_false", default)]
    pub discoverable: bool,
    /// Reserved; URL for handling interactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactions_url: Option<String>,
    /// URL for terms of service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_service_url: Option<String>,
    /// URL for privacy policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_policy_url: Option<String>,

    /// Enum of bot flags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<i32>,
}

/// Optional fields on bot object
#[derive(Serialize, Deserialize, JsonSchema, Debug, PartialEq, Eq)]
pub enum FieldsBot {
    Token,
    InteractionsURL,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum BotType {
    #[serde(rename = "custom_bot")]
    CustomBot,
    #[serde(rename = "prompt_bot")]
    PromptBot,
}

impl<'v> rocket::form::FromFormField<'v> for BotType {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        match field.value {
            "custom_bot" => Ok(BotType::CustomBot),
            "prompt_bot" => Ok(BotType::PromptBot),
            _ => Err(field.unexpected().into()),
        }
    }
}

impl BotType {
    pub fn as_str(&self) -> &str {
        match self {
            BotType::CustomBot => "custom_bot",
            BotType::PromptBot => "prompt_bot",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::form::{FromFormField, ValueField};

    #[test]
    fn test_bot_type_from_value() {
        let value_field = ValueField::from_value("custom_bot");
        let bot_type = BotType::from_value(value_field).unwrap();
        assert_eq!(bot_type, BotType::CustomBot);

        let value_field = ValueField::from_value("prompt_bot");
        let bot_type = BotType::from_value(value_field).unwrap();
        assert_eq!(bot_type, BotType::PromptBot);

        let value_field = ValueField::from_value("unexpected");
        let bot_type = BotType::from_value(value_field.clone());
        assert_eq!(bot_type, Err(value_field.unexpected().into()));
    }

    #[test]
    fn test_bot_type_as_str() {
        let custom_bot = BotType::CustomBot;
        let custom_bot_str = custom_bot.as_str();
        assert_eq!(custom_bot_str, "custom_bot");

        let prompt_bot = BotType::PromptBot;
        let prompt_bot_str = prompt_bot.as_str();
        assert_eq!(prompt_bot_str, "prompt_bot");
    }
}
