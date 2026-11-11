fn jei_hud_before(game: &mut crate::game::Game<'_>) {
    powah_optional::if_kit_jei!("powah", {
    crate::game::hz_kit_window_powah::hud_before(game);
} else { let _ = game; () })
}
