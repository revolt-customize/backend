use super::File;
use once_cell::sync::Lazy;
use regex::Regex;
use validator::Validate;

/// Regex for valid usernames
///
/// Block zero width space
/// Block lookalike characters
pub static RE_USERNAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\p{L}|[\d_.-])+$").unwrap());

auto_derived_partial_with_no_eq!(
    /// User
    pub struct User {
        /// Unique Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,
        /// Username
        pub username: String,
        /// Discriminator
        pub discriminator: String,
        /// Display name
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub display_name: Option<String>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        /// Avatar attachment
        pub avatar: Option<File>,
        /// Relationships with other users
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "Vec::is_empty", default)
        )]
        pub relations: Vec<Relationship>,

        /// Bitfield of user badges
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_zero_u32", default)
        )]
        pub badges: u32,
        /// User's current status
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub status: Option<UserStatus>,
        /// User's profile page
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub profile: Option<UserProfile>,

        /// Enum of user flags
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_zero_u32", default)
        )]
        pub flags: u32,
        /// Whether this user is privileged
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_false", default)
        )]
        pub privileged: bool,
        /// Bot information
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub bot: Option<BotInformation>,

        /// Current session user's relationship with this user
        pub relationship: RelationshipStatus,
        /// Whether this user is currently online
        pub online: bool,
    },
    "PartialUser"
);

auto_derived!(
    /// Optional fields on user object
    pub enum FieldsUser {
        Avatar,
        StatusText,
        StatusPresence,
        ProfileContent,
        ProfileBackground,
    }

    /// User's relationship with another user (or themselves)
    #[derive(Default)]
    pub enum RelationshipStatus {
        /// No relationship with other user
        #[default]
        None,
        /// Other user is us
        User,
        /// Friends with the other user
        Friend,
        /// Pending friend request to user
        Outgoing,
        /// Incoming friend request from user
        Incoming,
        /// Blocked this user
        Blocked,
        /// Blocked by this user
        BlockedOther,
    }

    /// Relationship entry indicating current status with other user
    pub struct Relationship {
        /// Other user's Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub user_id: String,
        /// Relationship status with them
        pub status: RelationshipStatus,
    }

    /// Presence status
    pub enum Presence {
        /// User is online
        Online,
        /// User is not currently available
        Idle,
        /// User is focusing / will only receive mentions
        Focus,
        /// User is busy / will not receive any notifications
        Busy,
        /// User appears to be offline
        Invisible,
    }

    /// User's active status
    pub struct UserStatus {
        /// Custom status text
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "String::is_empty"))]
        pub text: String,
        /// Current presence option
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub presence: Option<Presence>,
    }

    /// User's profile
    pub struct UserProfile {
        /// Text content on user's profile
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "String::is_empty"))]
        pub content: String,
        /// Background visible on user's profile
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub background: Option<File>,
    }

    /// User badge bitfield
    #[repr(u32)]
    pub enum UserBadges {
        /// Revolt Developer
        Developer = 1,
        /// Helped translate Revolt
        Translator = 2,
        /// Monetarily supported Revolt
        Supporter = 4,
        /// Responsibly disclosed a security issue
        ResponsibleDisclosure = 8,
        /// Revolt Founder
        Founder = 16,
        /// Platform moderator
        PlatformModeration = 32,
        /// Active monetary supporter
        ActiveSupporter = 64,
        /// 🦊🦝
        Paw = 128,
        /// Joined as one of the first 1000 users in 2021
        EarlyAdopter = 256,
        /// Amogus
        ReservedRelevantJokeBadge1 = 512,
        /// Low resolution troll face
        ReservedRelevantJokeBadge2 = 1024,
    }

    /// User flag enum
    #[repr(u32)]
    pub enum UserFlags {
        /// User has been suspended from the platform
        Suspended = 1,
        /// User has deleted their account
        Deleted = 2,
        /// User was banned off the platform
        Banned = 4,
        /// User was marked as spam and removed from platform
        Spam = 8,
    }

    #[derive(Default)]
    pub struct PromptTemplate {
        pub system_prompt: String,
        pub role_requirements: String,
    }
);

auto_derived_with_no_eq!(
    /// Bot information for if the user is a bot
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    #[derive(Default)]
    pub struct BotInformation {
        /// Id of the owner of this bot
        #[cfg_attr(feature = "serde", serde(rename = "owner"))]
        pub owner_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[validate]
        pub model: Option<BotModel>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub welcome: Option<String>,
    }

    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct BotModel {
        pub model_name: String,
        pub prompts: PromptTemplate,
        #[validate(range(min = 0.0, max = 1.0))]
        pub temperature: f32,
    }
);

impl Default for BotModel {
    fn default() -> Self {
        Self {
            model_name: "gpt-3.5-turbo".to_owned(),
            prompts: Default::default(),
            temperature: Default::default(),
        }
    }
}

pub trait CheckRelationship {
    fn with(&self, user: &str) -> RelationshipStatus;
}

impl CheckRelationship for Vec<Relationship> {
    fn with(&self, user: &str) -> RelationshipStatus {
        for entry in self {
            if entry.user_id == user {
                return entry.status.clone();
            }
        }

        RelationshipStatus::None
    }
}

#[cfg(test)]
#[cfg(feature = "validator")]
mod tests {

    use validator::Validate;

    use crate::v0::{BotInformation, BotModel, PromptTemplate};

    #[test]
    fn test_default_bot() {
        let bot_information = BotInformation {
            owner_id: "id1".into(),
            model: Some(Default::default()),
            welcome: None,
        };

        assert_eq!(
            bot_information.model.clone().unwrap(),
            BotModel {
                model_name: "gpt-3.5-turbo".into(),
                prompts: PromptTemplate {
                    system_prompt: "".into(),
                    role_requirements: "".into(),
                },
                temperature: 0.0,
            }
        );
    }

    #[test]
    #[cfg(feature = "validator")]
    fn test_validate() {
        let bot_model = BotModel {
            temperature: 1.4,
            ..Default::default()
        };

        assert!(bot_model.validate().is_err());
    }
}
