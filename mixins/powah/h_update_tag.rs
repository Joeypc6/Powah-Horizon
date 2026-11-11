fn powah_update_tag(entity: &horizon_client::containers::ModBlockEntity, tag: &mut Option<horizon_world::nbt::Compound>) {
    crate::game::powah::update_tag(entity, tag);
}
