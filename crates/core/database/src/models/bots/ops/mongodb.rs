use revolt_result::Result;

use crate::{Bot, FieldsBot, PartialBot};
use crate::{IntoDocumentPath, MongoDb};
use futures::StreamExt;

use super::AbstractBots;

static COL: &str = "bots";

#[async_trait]
impl AbstractBots for MongoDb {
    /// Insert new bot into the database
    async fn insert_bot(&self, bot: &Bot) -> Result<()> {
        query!(self, insert_one, COL, &bot).map(|_| ())
    }

    /// Fetch a bot by its id
    async fn fetch_bot(&self, id: &str) -> Result<Bot> {
        query!(self, find_one_by_id, COL, id)?.ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch a bot by its token
    async fn fetch_bot_by_token(&self, token: &str) -> Result<Bot> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "token": token
            }
        )?
        .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch bots owned by a user
    async fn fetch_bots_by_user(&self, user_id: &str) -> Result<Vec<Bot>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "owner": user_id
            }
        )
    }

    /// Get the number of bots owned by a user
    async fn get_number_of_bots_by_user(&self, user_id: &str) -> Result<usize> {
        query!(
            self,
            count_documents,
            COL,
            doc! {
                "owner": user_id
            }
        )
        .map(|v| v as usize)
    }

    /// Update bot with new information
    async fn update_bot(
        &self,
        id: &str,
        partial: &PartialBot,
        remove: Vec<FieldsBot>,
    ) -> Result<()> {
        query!(
            self,
            update_one_by_id,
            COL,
            id,
            partial,
            remove.iter().map(|x| x as &dyn IntoDocumentPath).collect(),
            None
        )
        .map(|_| ())
    }

    /// Delete a bot from the database
    async fn delete_bot(&self, id: &str) -> Result<()> {
        query!(self, delete_one_by_id, COL, id).map(|_| ())
    }

    async fn fetch_discoverable_bots(&self) -> Result<Vec<Bot>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "public": true
            }
        )
    }

    async fn search_bots_by_type(&self, bot_type: &str) -> Result<Vec<Bot>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "bot_type": bot_type
            }
        )
    }

    /// Fetch multiple bots by their ids
    async fn fetch_bots<'a>(&self, ids: &'a [String]) -> Result<Vec<Bot>> {
        Ok(self
            .col::<Bot>(COL)
            .find(
                doc! {
                    "_id": {
                        "$in": ids
                    }
                },
                None,
            )
            .await
            .map_err(|_| create_database_error!("find", COL))?
            .filter_map(|s| async {
                if cfg!(debug_assertions) {
                    Some(s.unwrap())
                } else {
                    s.ok()
                }
            })
            .collect()
            .await)
    }
}

impl IntoDocumentPath for FieldsBot {
    fn as_path(&self) -> Option<&'static str> {
        match self {
            FieldsBot::InteractionsURL => Some("interactions_url"),
            FieldsBot::Token => None,
        }
    }
}
