macro_rules! out {
    ($cnt: expr, $ctx: expr) => {
        $ctx.respond($cnt).await?;
        return Ok(());
    };
    ($cnt: expr, ?$ctx: expr) => {
        $ctx.respond($cnt).await?;
    };
}

// macro_rules! out_fol {
//     ($cnt: expr, $ctx: expr) => {
//         $ctx.followup(&$cnt).await?;
//         return Ok(());
//     };
//     ($cnt: expr, ?$ctx: expr) => {
//         $ctx.followup(&$cnt).await?;
//     };
// }

macro_rules! out_or_fol {
    ($cnt: expr, $ctx: expr) => {
        if $ctx.acknowledged() {
            $ctx.followup(&$cnt).await?;
            return Ok(());
        }
        $ctx.respond($cnt).await?;
        return Ok(());
    };
    ($cnt: expr, ?$ctx: expr) => {
        if $ctx.acknowledged() {
            $ctx.followup(&$cnt).await?;
        } else {
            $ctx.respond($cnt).await?;
        }
    };
}

macro_rules! out_or_upd {
    ($cnt: expr, $ctx: expr) => {
        if $ctx.acknowledged() {
            $ctx.update_no_components_embeds(&$cnt).await?;
            return Ok(());
        }
        $ctx.respond($cnt).await?;
        return Ok(());
    };
    ($cnt: expr, ?$ctx: expr) => {
        if $ctx.acknowledged() {
            $ctx.update_no_components_embeds(&$cnt).await?;
        } else {
            $ctx.respond($cnt).await?;
        }
    };
}

macro_rules! out_upd {
    ($cnt: expr, $ctx: expr) => {
        $ctx.update_no_components_embeds(&$cnt).await?;
        return Ok(());
    };
    ($cnt: expr, ?$ctx: expr) => {
        $ctx.update_no_components_embeds(&$cnt).await?;
    };
}

macro_rules! hid {
    ($cnt: expr, $ctx: expr) => {
        $ctx.ephem($cnt).await?;
        return Ok(());
    };
    ($cnt: expr, ?$ctx: expr) => {
        $ctx.ephem($cnt).await?;
    };
}

macro_rules! hid_fol {
    ($cnt: expr, $ctx: expr) => {
        $ctx.followup_ephem(&$cnt).await?;
        return Ok(());
    };
    ($cnt: expr, ?$ctx: expr) => {
        $ctx.followup_ephem(&$cnt).await?
    };
}

// macro_rules! hid_or_fol {
//     ($cnt: expr, $ctx: expr) => {
//         if $ctx.acknowledged() {
//             $ctx.followup_ephem(&$cnt).await?;
//             return Ok(());
//         }
//         $ctx.ephem($cnt).await?;
//         return Ok(());
//     };
//     ($cnt: expr, ?$ctx: expr) => {
//         if $ctx.acknowledged() {
//             $ctx.followup_ephem(&$cnt).await?;
//         } else {
//             $ctx.ephem($cnt).await?;
//         }
//     };
// }

macro_rules! generate_hid_variants {
    ($($name: ident => $emoji: ident),+$(,)?) => {
        $(
            macro_rules! $name {
                ($cnt: expr, $ctx: expr) => {
                    use crate::bot::core::r#const::exit_code;
                    hid!(format!("{} {}", exit_code::$emoji, $cnt), $ctx);
                };
                ($cnt: expr, ?$ctx: expr) => {
                    use crate::bot::core::r#const::exit_code;
                    hid!(format!("{} {}", exit_code::$emoji, $cnt), ?$ctx);
                };
            }
        )+

        pub(crate) use {$($name,)+};
    }
}

macro_rules! generate_hid_fol_variants {
    ($($name: ident => $emoji: ident),+$(,)?) => {
        $(
            macro_rules! $name {
                ($cnt: expr, $ctx: expr) => {
                    use crate::bot::core::r#const::exit_code;
                    hid_fol!(format!("{} {}", exit_code::$emoji, $cnt), $ctx);
                };
                ($cnt: expr, ?$ctx: expr) => {
                    {
                        use crate::bot::core::r#const::exit_code;
                        hid_fol!(format!("{} {}", exit_code::$emoji, $cnt), ?$ctx)
                    }
                };
            }
        )+

        pub(crate) use {$($name,)+};
    }
}

// macro_rules! generate_hid_or_fol_variants {
//     ($($name: ident => $emoji: ident),+$(,)?) => {
//         $(
//             macro_rules! $name {
//                 ($cnt: expr, $ctx: expr) => {
//                     use crate::bot::core::consts::exit_code;
//                     hid_or_fol!(format!("{} {}", exit_code::$emoji, $cnt), $ctx);
//                 };
//                 ($cnt: expr, ?$ctx: expr) => {
//                     use crate::bot::core::consts::exit_code;
//                     hid_or_fol!(format!("{} {}", exit_code::$emoji, $cnt), ?$ctx);
//                 };
//             }
//         )+

//         pub(crate) use {$($name,)+};
//     }
// }

generate_hid_variants! {
    note => NOTICE,
    sus => DUBIOUS,
    caut => WARNING,
    what => NOT_FOUND,
    bad => INVALID,
    nope => PROHIBITED,
    cant => FORBIDDEN,
    err => KNOWN_ERROR,
    crit => UNKNOWN_ERROR
}

generate_hid_fol_variants! {
    note_fol => NOTICE,
    sus_fol => DUBIOUS,
    // caut_fol => WARNING,
    // miss_fol => NOT_FOUND,
    // bad_fol => INVALID,
    // nope_fol => PROHIBITED,
    // cant_fol => FORBIDDEN,
    // err_fol => KNOWN_ERROR,
    // crit_fol => UNKNOWN_ERROR
}

// generate_hid_or_fol_variants! {
//     note_or_fol => NOTICE,
//     dub_or_fol => DUBIOUS,
//     caut_or_fol => WARNING,
//     miss_or_fol => NOT_FOUND,
//     bad_or_fol => INVALID,
//     nope_or_fol => PROHIBITED,
//     cant_or_fol => FORBIDDEN,
//     err_or_fol => KNOWN_ERROR,
//     crit_or_fol => UNKNOWN_ERROR
// }

pub(crate) use {hid, hid_fol, out, out_or_fol, out_or_upd, out_upd};
