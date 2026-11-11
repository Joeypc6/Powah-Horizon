fn kit_guide_released_powah(game: &crate::game::Game<'_>, canceled: &mut bool) {
    powah_optional::if_kit!("powah", {
    crate::game::hz_kit_guide_powah::release(game, canceled);
} else { let _ = (game, canceled); () })
}
