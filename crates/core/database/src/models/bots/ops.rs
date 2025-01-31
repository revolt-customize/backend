use revolt_result::Result;

use crate::{Bot, FieldsBot, PartialBot};

mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractBots: Sync + Send {
    /// Insert new bot into the database
    async fn insert_bot(&self, bot: &Bot) -> Result<()>;

    /// Fetch a bot by its id
    async fn fetch_bot(&self, id: &str) -> Result<Bot>;

    /// Fetch multiple bots by their ids
    async fn fetch_bots<'a>(&self, ids: &'a [String]) -> Result<Vec<Bot>>;

    /// Fetch a bot by its token
    async fn fetch_bot_by_token(&self, token: &str) -> Result<Bot>;

    /// Fetch bots owned by a user
    async fn fetch_bots_by_user(&self, user_id: &str) -> Result<Vec<Bot>>;

    /// Get the number of bots owned by a user
    async fn get_number_of_bots_by_user(&self, user_id: &str) -> Result<usize>;

    /// Update bot with new information
    async fn update_bot(
        &self,
        id: &str,
        partial: &PartialBot,
        remove: Vec<FieldsBot>,
    ) -> Result<()>;

    /// Delete a bot from the database
    async fn delete_bot(&self, id: &str) -> Result<()>;

    async fn fetch_discoverable_bots(&self) -> Result<Vec<Bot>>;

    async fn search_bots_by_type(&self, bot_type: &str) -> Result<Vec<Bot>>;
}
