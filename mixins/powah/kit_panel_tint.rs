fn jei_panel_tint(game: &Game<'_>, batch: &mut HudBatch) {
    powah_optional::if_kit_jei!("powah", {
    crate::game::hz_kit_window_powah::tint(game, batch);
} else { let _ = (game, batch); () })
}
