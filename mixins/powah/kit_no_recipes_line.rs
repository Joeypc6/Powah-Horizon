// `native_chat`: holds back JEI's "No recipes for" while the window shows more than JEI's own recipes.
fn jei_no_recipes_line(text: &str) -> core::ops::ControlFlow<()> {
    powah_optional::if_kit_jei!("powah", {
    if crate::game::hz_kit_layer_powah::chat_allowed(text) {
        core::ops::ControlFlow::Continue(())
    } else {
        core::ops::ControlFlow::Break(())
    }
} else { let _ = text; core::ops::ControlFlow::Continue(()) })
}
