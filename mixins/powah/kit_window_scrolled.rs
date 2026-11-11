fn jei_window_scrolled(game: &crate::game::Game<'_>, scroll: [f64; 2], canceled: &mut bool) {
    powah_optional::if_kit_jei!("powah", {
    if game.player.is_none() || *canceled {
        return;
    }
    crate::game::hz_kit_window_powah::scroll(scroll[1], canceled);
} else { let _ = (game, scroll, canceled); () })
}
