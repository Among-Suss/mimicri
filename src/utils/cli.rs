use poise::serenity_prelude as serenity;
use std::env;

pub async fn parse_args() -> Option<i32> {
    let args: Vec<String> = env::args().collect();

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let guild_id = env::var("DEBUG_GUILD_ID")
        .expect("No guild to deregister from. Set DEBUG_GUILD_ID")
        .parse()
        .unwrap();

    let client = serenity::Client::builder(&env::var("DISCORD_TOKEN").unwrap(), intents)
        .await
        .unwrap();

    let http = &client.cache_and_http.http;

    let _guild_name = http.get_guild(guild_id).await.unwrap().name;

    if args.len() > 0 {
        let mut exit_code = 0;
        for arg in &args[1..args.len()] {
            match arg.as_str() {
                &_ => {
                    println!("Unknown argument: {}", arg);
                    exit_code = 1
                }
            }
        }

        Some(exit_code)
    } else {
        None
    }
}
