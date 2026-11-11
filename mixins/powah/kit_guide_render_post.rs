fn kit_guide_render_post_powah(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, screen: &crate::game::native_hooks::NativeScreenView<'_>) {
    powah_optional::if_kit!("powah", {
    if game.player.is_none() {
        return;
    }
    crate::game::hz_kit_guide_powah::screen_frame(screen.id, screen.menu);
    crate::game::hz_kit_guide_powah::draw(game, batch, screen.mouse);
} else { let _ = (game, batch, screen); () })
}
