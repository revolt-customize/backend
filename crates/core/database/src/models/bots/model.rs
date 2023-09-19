use std::collections::HashMap;

use revolt_permissions::DEFAULT_PERMISSION_SERVER;
use revolt_result::Result;
use ulid::Ulid;

use crate::{Channel, Database, Invite, Member, PartialServer, PartialUser, Server, User};

auto_derived_partial!(
    /// Bot
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
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub analytics: bool,
        /// Whether this bot should be publicly discoverable
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub discoverable: bool,
        /// Reserved; URL for handling interactions
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub interactions_url: String,
        /// URL for terms of service
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub terms_of_service_url: String,
        /// URL for privacy policy
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub privacy_policy_url: String,

        /// Enum of bot flags
        #[serde(skip_serializing_if = "Option::is_none")]
        pub flags: Option<i32>,

        /// Bot server invite code
        #[serde(skip_serializing_if = "Option::is_none")]
        pub server_invite: Option<String>,

        /// Bot's default server
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_server: Option<String>,
    },
    "PartialBot"
);

auto_derived!(
    /// Optional fields on bot object
    pub enum FieldsBot {
        Token,
        InteractionsURL,
    }

    pub enum BotType {
        #[serde(rename = "custom_bot")]
        CustomBot,
        #[serde(rename = "prompt_bot")]
        PromptBot,
    }
);

impl BotType {
    pub fn as_str(&self) -> &str {
        match self {
            BotType::CustomBot => "custom_bot",
            BotType::PromptBot => "prompt_bot",
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Bot {
    fn default() -> Self {
        Self {
            id: Default::default(),
            owner: Default::default(),
            token: Default::default(),
            public: Default::default(),
            analytics: Default::default(),
            discoverable: Default::default(),
            interactions_url: Default::default(),
            terms_of_service_url: Default::default(),
            privacy_policy_url: Default::default(),
            flags: Default::default(),
            bot_type: Default::default(),
            server_invite: Default::default(),
            default_server: Default::default(),
        }
    }
}

#[allow(clippy::disallowed_methods)]
impl Bot {
    /// Create a new bot
    pub async fn create<D>(db: &Database, username: String, owner: &User, data: D) -> Result<Bot>
    where
        D: Into<Option<PartialBot>>,
    {
        // TODO: config
        let max_bot_count = 10;
        if db.get_number_of_bots_by_user(&owner.id).await? >= max_bot_count {
            return Err(create_error!(ReachedMaximumBots));
        }

        let id = Ulid::new().to_string();

        User::create(
            db,
            username,
            Some(id.to_string()),
            Some(PartialUser {
                bot: owner.bot.clone(),
                ..Default::default()
            }),
        )
        .await?;

        let mut bot = Bot {
            id,
            owner: owner.id.to_string(),
            token: nanoid::nanoid!(64),
            ..Default::default()
        };

        if let Some(data) = data.into() {
            bot.apply_options(data);
        }

        db.insert_bot(&bot).await?;
        Ok(bot)
    }

    /// Remove a field from this object
    pub fn remove_field(&mut self, field: &FieldsBot) {
        match field {
            FieldsBot::Token => self.token = nanoid::nanoid!(64),
            FieldsBot::InteractionsURL => {
                self.interactions_url = String::new();
            }
        }
    }

    /// Update this bot
    pub async fn update(
        &mut self,
        db: &Database,
        mut partial: PartialBot,
        remove: Vec<FieldsBot>,
    ) -> Result<()> {
        if remove.contains(&FieldsBot::Token) {
            partial.token = Some(nanoid::nanoid!(64));
        }

        for field in &remove {
            self.remove_field(field);
        }

        db.update_bot(&self.id, &partial, remove).await?;

        self.apply_options(partial);
        Ok(())
    }

    /// Delete this bot
    pub async fn delete(&self, db: &Database) -> Result<()> {
        db.fetch_user(&self.id).await?.mark_deleted(db).await?;
        db.delete_bot(&self.id).await?;

        if let Some(ref default_server) = self.default_server {
            let mut server = db.fetch_server(default_server).await?;
            server
                .update(
                    db,
                    PartialServer {
                        name: Some(format!("{} (deleted)", server.name)),
                        ..Default::default()
                    },
                    vec![],
                )
                .await?;

            db.delete_member(&crate::MemberCompositeKey {
                server: server.id.clone(),
                user: self.id.clone(),
            })
            .await?;
        }

        Ok(())
    }
}

impl Bot {
    pub async fn prepare_default_channel_for_bot(
        db: &Database,
        bot: &mut Bot,
        bot_user: &User,
        bot_owner: &User,
    ) -> Result<()> {
        let server_id = Ulid::new().to_string();
        let mut channels: Vec<Channel> = Vec::new();
        for channel_name in ["BOT使用新手指南", "功能发布", "bug反馈", "大家一起玩"]
        {
            let channel = Channel::TextChannel {
                id: Ulid::new().to_string(),
                server: server_id.clone(),
                name: channel_name.into(),
                description: None,
                icon: None,
                last_message_id: None,
                default_permissions: None,
                role_permissions: HashMap::new(),
                nsfw: false,
            };

            channel.create(db).await?;
            channels.push(channel);
        }

        let server = Server {
            id: server_id,
            owner: bot.owner.clone(),
            name: format!("{}的主页", bot_user.username),
            description: None,
            channels: channels.iter().map(|x| x.id()).collect(),
            nsfw: false,
            default_permissions: *DEFAULT_PERMISSION_SERVER as i64,
            ..Default::default()
        };

        server.create(db).await?;

        Member::create(db, &server, bot_owner).await?;
        Member::create(db, &server, bot_user).await?;

        let mut invite_code: Option<String> = None;
        let mut default_server: Option<String> = None;

        if let Invite::Server { code, server, .. } =
            Invite::create_channel_invite(db, bot.owner.clone(), channels.first().unwrap()).await?
        {
            invite_code = Some(code);
            default_server = Some(server)
        }

        bot.update(
            db,
            PartialBot {
                server_invite: invite_code,
                default_server,
                ..Default::default()
            },
            vec![],
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Bot, BotType, FieldsBot, Invite, PartialBot, User};

    #[async_std::test]
    async fn crud() {
        database_test!(|db| async move {
            let mut owner = User::create(&db, "Owner".to_string(), None, None)
                .await
                .unwrap();

            owner.bot = Some(crate::BotInformation {
                owner: owner.id.clone(),
                welcome: None,
                model: Some(crate::BotModel {
                    model_name: "gpt-4".into(),
                    prompts: crate::PromptTemplate {
                        system_prompt: "system".into(),
                        role_requirements: None,
                    },
                    temperature: 0.4,
                }),
            });

            let bot = Bot::create(
                &db,
                "Bot Name".to_string(),
                &owner,
                PartialBot {
                    token: Some("my token".to_string()),
                    interactions_url: Some("some url".to_string()),
                    bot_type: Some(BotType::PromptBot),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

            assert!(!bot.interactions_url.is_empty());

            let mut updated_bot = bot.clone();
            updated_bot
                .update(
                    &db,
                    PartialBot {
                        public: Some(true),
                        ..Default::default()
                    },
                    vec![FieldsBot::Token, FieldsBot::InteractionsURL],
                )
                .await
                .unwrap();

            let fetched_bot1 = db.fetch_bot(&bot.id).await.unwrap();
            let fetched_bot2 = db.fetch_bot_by_token(&fetched_bot1.token).await.unwrap();
            let fetched_bots = db.fetch_bots_by_user(&owner.id).await.unwrap();
            let fetched_bots2 = db.fetch_bots(&[bot.id.clone()]).await.unwrap();

            assert_eq!(1, fetched_bots2.len());
            assert_eq!(fetched_bots, fetched_bots2);

            let fetched_user_bot = db.fetch_user(&bot.id).await.unwrap();
            assert_eq!(fetched_user_bot.bot, owner.bot);

            assert!(!bot.public);
            assert!(fetched_bot1.public);
            assert!(!bot.interactions_url.is_empty());
            assert!(fetched_bot1.interactions_url.is_empty());
            assert_ne!(bot.token, fetched_bot1.token);
            assert_eq!(updated_bot, fetched_bot1);
            assert_eq!(fetched_bot1, fetched_bot2);
            assert_eq!(fetched_bot1, fetched_bots[0]);
            assert_eq!(1, db.get_number_of_bots_by_user(&owner.id).await.unwrap());

            bot.delete(&db).await.unwrap();
            assert!(db.fetch_bot(&bot.id).await.is_err());
            assert_eq!(0, db.get_number_of_bots_by_user(&owner.id).await.unwrap());
            assert_eq!(db.fetch_user(&bot.id).await.unwrap().flags, Some(2))
        });
    }

    #[async_std::test]
    async fn test_prepare_default_channel_for_bot() {
        database_test!(|db| async move {
            let mut owner = User::create(&db, "Owner".to_string(), None, None)
                .await
                .unwrap();

            owner.bot = Some(crate::BotInformation {
                owner: owner.id.clone(),
                welcome: None,
                model: Some(crate::BotModel {
                    model_name: "gpt-4".into(),
                    prompts: crate::PromptTemplate {
                        system_prompt: "system".into(),
                    },
                    temperature: 0.4,
                }),
            });

            let mut bot = Bot::create(
                &db,
                "Bot Name".to_string(),
                &owner,
                PartialBot {
                    token: Some("my token".to_string()),
                    interactions_url: Some("some url".to_string()),
                    bot_type: Some(BotType::PromptBot),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

            assert!(bot.default_server.is_none());
            assert!(bot.server_invite.is_none());

            let bot_user = db.fetch_user(&bot.id).await.unwrap();

            Bot::prepare_default_channel_for_bot(&db, &mut bot, &bot_user, &owner)
                .await
                .unwrap();

            assert!(bot.server_invite.is_some());
            assert!(bot.default_server.is_some());

            let server = db.fetch_server(&bot.default_server.unwrap()).await.unwrap();
            assert_eq!(4, server.channels.len());
            let channels = db.fetch_channels(&server.channels).await.unwrap();
            assert_eq!(4, channels.len());

            let invite = db
                .fetch_invite(bot.server_invite.as_ref().unwrap())
                .await
                .unwrap();

            match invite {
                Invite::Server {
                    code,
                    server: _server,
                    channel,
                    ..
                } => {
                    assert_eq!(Some(code), bot.server_invite);
                    assert_eq!(_server, server.id);
                    assert_eq!(channel, channels[0].id());
                }
                _ => unreachable!(),
            }
        });
    }
}
