fn kit_guide_shown_powah(stack: &ItemStack) {
    powah_optional::if_kit!("powah", {
    crate::game::hz_kit_guide_powah::hovered(&stack.item);
} else { let _ = stack; () })
}
