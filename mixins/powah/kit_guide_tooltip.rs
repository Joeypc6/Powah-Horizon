fn kit_guide_tooltip_powah(stack: &horizon_client::items::ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    powah_optional::if_kit!("powah", {
    crate::game::hz_kit_guide_powah::tooltip(stack, lines);
} else { let _ = (stack, lines); () })
}
