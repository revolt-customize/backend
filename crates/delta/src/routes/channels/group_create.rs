use std::{collections::HashSet, iter::FromIterator};

use revolt_models::v0;
use revolt_quark::{
    get_relationship,
    models::user::{RelationshipStatus, User},
    variables::delta::{MAX_GROUP_SIZE, OFFICIAL_MODEL_BOTS},
};

use revolt_database::{Channel, Database};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use serde::{Deserialize, Serialize};
use ulid::Ulid;
use validator::Validate;

/// # Group Data
#[derive(Validate, Serialize, Deserialize, JsonSchema)]
pub struct DataCreateGroup {
    /// Group name
    #[validate(length(min = 1, max = 32))]
    name: String,
    /// Group description
    #[validate(length(min = 0, max = 1024))]
    description: Option<String>,
    /// Array of user IDs to add to the group
    ///
    /// Must be friends with these users.
    #[validate(length(min = 0, max = 49))]
    users: Vec<String>,
    /// Whether this group is age-restricted
    #[serde(skip_serializing_if = "Option::is_none")]
    nsfw: Option<bool>,
}

/// # Create Group
///
/// Create a new group channel.
#[openapi(tag = "Groups")]
#[post("/create", data = "<info>")]
pub async fn req(
    db: &State<Database>,
    user: User,
    info: Json<DataCreateGroup>,
) -> Result<Json<v0::Channel>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let info = info.into_inner();
    info.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut set: HashSet<String> = HashSet::from_iter(info.users.into_iter());
    set.insert(user.id.clone());

    if set.len() > *MAX_GROUP_SIZE {
        return Err(create_error!(GroupTooLarge {
            max: *MAX_GROUP_SIZE,
        }));
    }

    for target in &set {
        match get_relationship(&user, target) {
            RelationshipStatus::Friend | RelationshipStatus::User => {}
            _ => {
                return Err(create_error!(NotFriends));
            }
        }
    }

    let mut group = Channel::Group {
        id: Ulid::new().to_string(),
        name: info.name,
        owner: user.id.clone(),
        description: info.description,
        recipients: set.into_iter().collect::<Vec<String>>(),

        icon: None,
        last_message_id: None,

        permissions: None,

        nsfw: info.nsfw.unwrap_or(false),
    };

    group.create(db).await?;

    add_official_prompt_bots(db, user.id.clone(), &mut group).await?;

    Ok(Json(group.into()))
}

/// add official prompts bot for any new created group
async fn add_official_prompt_bots(
    db: &Database,
    user_id: String,
    group: &mut Channel,
) -> Result<()> {
    if (*OFFICIAL_MODEL_BOTS).is_empty() {
        return Ok(());
    }

    for bot in db.fetch_users(OFFICIAL_MODEL_BOTS.as_slice()).await? {
        group.add_user_to_group(&db.clone(), &bot, &user_id).await?;
    }

    Ok(())
}
