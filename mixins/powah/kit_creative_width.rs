// `native_screen_width`: the creative window's width while the panel works for it.
fn jei_creative_width() -> core::ops::ControlFlow<Option<f32>> {
    powah_optional::if_kit_jei!("powah", {
    match crate::game::hz_kit_creative_powah::screen_width() {
        Some(width) => core::ops::ControlFlow::Break(Some(width)),
        None => core::ops::ControlFlow::Continue(()),
    }
} else { core::ops::ControlFlow::Continue(()) })
}
