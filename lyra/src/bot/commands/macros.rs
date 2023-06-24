/// Normal interaction response
macro_rules! out {
    ($cnt: expr, $ctx: expr) => {
        $ctx.respond($cnt).await?;
        return Ok(());
    };
    ($cnt: expr, $ctx: expr, !) => {
        $ctx.respond($cnt).await?;
    };
}

/// Ephemeral interaction response
macro_rules! hid {
    ($cnt: expr, $ctx: expr) => {
        $ctx.ephem($cnt).await?;
        return Ok(());
    };
    ($cnt: expr, $ctx: expr, !) => {
        $ctx.ephem($cnt).await?;
    };
}

macro_rules! generate_hid_variants {
    ($($name: ident => $emoji: ident),+$(,)?) => {
        $(
            macro_rules! $name {
                ($cnt: expr, $ctx: expr) => {
                    use crate::bot::lib::consts::exit_codes;
                    hid!(format!("{} {}", exit_codes::$emoji, $cnt), $ctx);
                };
                ($cnt: expr, $ctx: expr, !) => {
                    use crate::bot::lib::consts::exit_codes;
                    hid!(format!("{} {}", exit_codes::$emoji, $cnt), $ctx, !);
                };
            }
        )+

        pub(crate) use {$($name,)+};
    }
}

generate_hid_variants! {
    note => NOTICE,
    dub => DUBIOUS,
    caut => WARNING,
    miss => NOT_FOUND,
    bad => INVALID,
    nope => PROHIBITED,
    cant => FORBIDDEN,
    err => KNOWN_ERROR,
    crit => UNKNOWN_ERROR
}

pub(crate) use {hid, out};
