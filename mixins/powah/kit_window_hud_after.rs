fn jei_hud_after(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) {
    powah_optional::if_kit_jei!("powah", {
    crate::game::hz_kit_window_powah::hud_after(game, batch);
} else { let _ = (game, batch); () })
}
