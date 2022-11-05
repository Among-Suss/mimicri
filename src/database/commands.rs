use std::cmp;

use serenity::{
    framework::standard::{Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::utils::{config, message_context::MessageContext, strings};

use super::plugin::get_db_plugin;

pub async fn history(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let message_ctx = MessageContext::new(ctx, msg);

    let page = cmp::max(args.single::<i64>().unwrap_or_default() - 1, 0) as usize;

    let guild_id = msg.guild(&ctx.cache).unwrap().id;

    let page_size = config::queue::page_size(guild_id);
    let queue_text_len = config::queue::text_length(guild_id);

    if let Some(db_plugin) = get_db_plugin(ctx).await {
        if let Ok((history, count)) =
            db_plugin.get_history(msg.author.id, page_size, page * page_size)
        {
            let mut description = String::new();

            for (i, info) in history.iter().enumerate() {
                description += &format!(
                    "{}. **{}**  [↗️]({})\n",
                    i + 1,
                    strings::escape_string(&strings::limit_string_length(
                        &info.title,
                        queue_text_len
                    )),
                    info.url
                )
                .to_string();
            }
            message_ctx
                .send_message(|m| {
                    m.content("").embed(|e| {
                        MessageContext::format_embed_playlist(
                            e,
                            history.iter(),
                            count,
                            guild_id,
                            page,
                        )
                        .title(format!("{}'s History", msg.author.name))
                        .color(config::colors::history())
                    });

                    m
                })
                .await;
        } else {
            message_ctx
                .send_error("Database error, unable to fetch history.")
                .await;
        }
    }
    Ok(())
}

pub async fn play_history(ctx: &Context, msg: &Message, mut args: Args) -> Option<String> {
    let message_ctx = MessageContext::new(ctx, msg);

    let no = cmp::max(args.single::<i64>().unwrap_or_default() - 1, 0) as usize;

    if let Some(db_plugin) = get_db_plugin(ctx).await {
        if let Ok((history, count)) = db_plugin.get_history(msg.author.id, 1, no) {
            if history.len() > 0 {
                return Some(history[0].url.clone());
            } else {
                message_ctx
                    .reply_warn(
                        msg,
                        format!("Song index not found. History contains {} songs.", count),
                    )
                    .await;
            }
        } else {
            message_ctx.reply_error(msg, "Unable to load history").await;
        }
    }

    None
}

pub mod interaction {
    use serenity::{
        model::prelude::{
            component::{ActionRowComponent, InputTextStyle},
            interaction::{
                application_command::ApplicationCommandInteraction, modal::ModalSubmitInteraction,
                InteractionResponseType,
            },
        },
        prelude::Context,
    };
    use tracing::error;

    use crate::database::plugin::get_db_plugin;

    pub mod playlist_create {
        use super::*;

        pub const COMMAND_NAME: &str = "playlist";
        pub const SUBMIT_ID: &str = "createPlaylist.Modal";

        pub async fn modal(ctx: Context, command: ApplicationCommandInteraction) {
            if let Err(err) = command
                .create_interaction_response(&ctx.http, |r| {
                    r.kind(InteractionResponseType::Modal)
                        .interaction_response_data(|m| {
                            m.title("Create Playlist")
                                .custom_id(SUBMIT_ID)
                                .components(|c| {
                                    c.create_action_row(|r| {
                                        r.create_input_text(|i| {
                                            i.custom_id("create_playlist.input")
                                                .label("Playlist name:")
                                                .style(InputTextStyle::Short)
                                        })
                                    })
                                })
                        })
                })
                .await
            {
                error!("Unable to create playlist modal: {:?}", err);
            }
        }

        pub async fn submit(ctx: Context, interaction: ModalSubmitInteraction) {
            if let ActionRowComponent::InputText(input) =
                &interaction.data.components[0].components[0]
            {
                let playlist_name = &input.value;

                if let Some(db_plugin) = get_db_plugin(&ctx).await {
                    let _ = db_plugin.create_playlist(interaction.user.id, playlist_name);

                    if let Err(err) = interaction
                        .create_interaction_response(&ctx.http, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|d| {
                                    d.content(format!("Created playlist: {}", playlist_name))
                                })
                        })
                        .await
                    {
                        error!("Unable to submit playlist modal: {:?}", err);
                    }
                }
            }
        }
    }

    pub mod playlist_list {
        use super::*;

        pub const COMMAND_NAME: &str = "list-playlist";

        pub async fn response(ctx: Context, command: ApplicationCommandInteraction) {
            if let Some(db_plugin) = get_db_plugin(&ctx).await {
                if let Ok(playlists) = db_plugin.get_playlists(command.user.id, 10, 0) {
                    if let Err(err) = command
                        .create_interaction_response(&ctx.http, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|d| {
                                    d.embed(|e| {
                                        e.title(format!("{}'s playlists", command.user.name))
                                            .description(if !playlists.is_empty() {
                                                playlists
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(i, playlist)| {
                                                        format!("{}. **{}**", i + 1, playlist)
                                                    })
                                                    .collect::<Vec<String>>()
                                                    .join("\n")
                                            } else {
                                                format!("You don't have any playlists yet.")
                                            })
                                    })
                                })
                        })
                        .await
                    {
                        error!("Unable to create playlist modal: {:?}", err);
                    }
                }
            }
        }
    }
}
