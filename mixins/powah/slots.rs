// A container's stack replaced where it lies, for Powah's energy hopper (EnergyHopperTile charges items in their
// slots). Horizon's item helpers take items out and put them in; this swaps one stack for its charged copy in place,
// only if the slot still holds the stack it was read with.
impl crate::game::Game<'_> {
    pub(in crate::game) fn powah_replace_slot(&mut self, dimension: &str, pos: [i32; 3], slot: usize, expected: &horizon_client::items::ItemStack, new: horizon_client::items::ItemStack) -> bool {
        let Some(view) = self.native_item_source(dimension, pos) else { return false };
        let Some(target) = self.slot_mut_at(&view, slot) else { return false };
        if target.as_ref() != Some(expected) {
            return false;
        }
        *target = Some(new);
        self.view_changed(&view);
        true
    }
}
