use super::users::BotModel;
use super::User;
use validator::Validate;

auto_derived!(
    /// Bot
    #[derive(Default)]
    pub struct Bot {
        /// Bot Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,

        /// User Id of the bot owner
        #[cfg_attr(feature = "serde", serde(rename = "owner"))]
        pub owner_id: String,
        /// Token used to authenticate requests for this bot
        pub token: String,
        /// Whether the bot is public
        /// (may be invited by anyone)
        pub public: bool,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub bot_type: Option<BotType>,

        /// Whether to enable analytics
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_false", default)
        )]
        pub analytics: bool,
        /// Whether this bot should be publicly discoverable
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_false", default)
        )]
        pub discoverable: bool,
        /// Reserved; URL for handling interactions
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "String::is_empty", default)
        )]
        pub interactions_url: String,
        /// URL for terms of service
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "String::is_empty", default)
        )]
        pub terms_of_service_url: String,
        /// URL for privacy policy
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "String::is_empty", default)
        )]
        pub privacy_policy_url: String,

        /// Enum of bot flags
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_zero_u32", default)
        )]
        pub flags: u32,

        /// Bot server invite code
        #[serde(skip_serializing_if = "Option::is_none")]
        pub server_invite: Option<String>,

        /// Bot's default server
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_server: Option<String>,
    }

    /// Optional fields on bot object
    pub enum FieldsBot {
        Token,
        InteractionsURL,
    }

    /// Flags that may be attributed to a bot
    #[repr(u32)]
    pub enum BotFlags {
        Verified = 1,
        Official = 2,
    }

    /// Public Bot
    pub struct PublicBot {
        /// Bot Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,

        /// Bot Username
        pub username: String,
        /// Profile Avatar
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "String::is_empty", default)
        )]
        pub avatar: String,
        /// Profile Description
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "String::is_empty", default)
        )]
        pub description: String,
    }

    pub enum BotType {
        #[serde(rename = "custom_bot")]
        CustomBot,
        #[serde(rename = "prompt_bot")]
        PromptBot,
    }

    /// New Bot Details
    #[derive(Default)]
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct DataEditBot {
        /// Bot username
        #[cfg_attr(
            feature = "validator",
            validate(length(min = 2, max = 32), regex = "super::RE_USERNAME")
        )]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        /// Whether the bot can be added by anyone
        pub public: Option<bool>,
        /// Whether analytics should be gathered for this bot
        ///
        /// Must be enabled in order to show up on [Revolt Discover](https://rvlt.gg).
        pub analytics: Option<bool>,
        /// Interactions URL
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
        pub interactions_url: Option<String>,
        /// Fields to remove from bot object
        #[cfg_attr(feature = "validator", validate(length(min = 1)))]
        pub remove: Option<Vec<FieldsBot>>,
    }

    /// Where we are inviting a bot to
    #[serde(untagged)]
    pub enum InviteBotDestination {
        /// Invite to a server
        Server {
            /// Server Id
            server: String,
        },
        /// Invite to a group
        Group {
            /// Group Id
            group: String,
        },
    }
);

auto_derived_with_no_eq!(
    /// Bot Response
    pub struct FetchBotResponse {
        /// Bot object
        pub bot: Bot,
        /// User object
        pub user: User,
    }

    /// Bot Details
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct DataCreateBot {
        /// Bot username
        #[cfg_attr(
            feature = "validator",
            validate(length(min = 2, max = 32), regex = "super::RE_USERNAME")
        )]
        pub name: String,
        pub welcome: Option<String>,
        pub bot_type: Option<BotType>,
        #[cfg_attr(feature = "validator", validate)]
        pub model: Option<BotModel>,
    }

    /// Owned Bots Response
    ///
    /// Both lists are sorted by their IDs.
    ///
    /// TODO: user should be in bot object
    pub struct OwnedBotsResponse {
        /// Bot objects
        pub bots: Vec<Bot>,
        /// User objects
        pub users: Vec<User>,
    }
);

#[cfg(test)]
#[cfg(feature = "validator")]
mod tests {
    use crate::v0::{BotModel, BotType, DataCreateBot, PromptTemplate};
    use validator::Validate;

    #[test]
    #[cfg(feature = "validator")]
    fn test_validate() {
        let mut bot = DataCreateBot {
            name: "mybot".into(),
            welcome: None,
            bot_type: Some(BotType::PromptBot),
            model: Some(BotModel {
                model_name: "gpt4".into(),
                prompts: PromptTemplate {
                    system_prompt: "".into(),
                },
                temperature: 2.0,
            }),
        };

        assert!(bot.validate().map_err(|e| println!("{e}")).is_err());

        bot.model.as_mut().unwrap().temperature = 0.5;
        assert!(bot.validate().map_err(|e| println!("{e}")).is_ok());
    }
}
