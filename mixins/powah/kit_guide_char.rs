fn kit_guide_char_powah(game: &crate::game::Game<'_>, c: char, canceled: &mut bool) {
    powah_optional::if_kit!("powah", {
    let _ = c;
    if game.player.is_some() && crate::game::hz_kit_guide_powah::is_open() {
        *canceled = true;
    }
} else { let _ = (game, c, canceled); () })
}
