use serenity::model::prelude::*;
use serenity::prelude::*;

pub async fn voice_state_update(ctx: Context, old: Option<VoiceState>, new: VoiceState) {
    if old.is_none() {
        return;
    }

    let old = old.unwrap();

    if old.channel_id.is_none() || new.channel_id.is_some() {
        return;
    }

    let guild_id = match new.guild_id {
        Some(id) => id,
        None => {
            return;
        }
    };

    let channel_id = old.channel_id.unwrap();
    let channel = match channel_id.to_channel(&ctx).await {
        Ok(channel) => channel,
        Err(e) => {
            eprintln!("Cannot get channel: {}", e);
            return;
        }
    };

    match channel {
        Channel::Guild(channel) => match channel.members(&ctx).await {
            Ok(members) => {
                if members.len() > 1 {
                    return;
                }
            }
            Err(e) => {
                eprintln!("Cannot get members: {}", e);
                return;
            }
        },
        _ => {
            return;
        }
    }

    let manager = match songbird::get(&ctx).await {
        Some(manager) => manager.clone(),
        None => {
            return;
        }
    };

    let has_handler = manager.get(guild_id).is_some();

    if !has_handler {
        return;
    }

    match manager.remove(guild_id).await {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Cannot remove handler: {}", e);
            return;
        }
    };
}
