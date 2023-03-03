macro_rules! handle {
    ($msg:expr, $bot:expr) => {{
        if $msg.guild_id.is_none() || !$msg.content.starts_with('!') {
            continue;
        }

        let ctx = Context::new($msg.clone().0, Arc::clone(&$bot));

        match $msg.content.split_whitespace().next() {
            Some("!join") => spawn(join(ctx)),
            Some("!leave") => spawn(leave(ctx)),
            Some("!pause") => spawn(pause(ctx)),
            Some("!play") => spawn(play(ctx)),
            Some("!seek") => spawn(seek(ctx)),
            Some("!stop") => spawn(stop(ctx)),
            Some("!volume") => spawn(volume(ctx)),
            _ => continue,
        }
    }};
}

pub(crate) use handle;
