fn kit_guide_scrolled_powah(game: &crate::game::Game<'_>, scroll: [f64; 2], canceled: &mut bool) {
    powah_optional::if_kit!("powah", {
    if game.player.is_some() {
        crate::game::hz_kit_guide_powah::scroll(scroll[1], canceled);
    }
} else { let _ = (game, scroll, canceled); () })
}
