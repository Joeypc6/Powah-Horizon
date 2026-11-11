fn powah_be_energy(block: &str, energy: &mut Option<horizon_client::containers::ModEnergyBuffer>) {
    crate::game::powah::block_entity_energy(block, energy);
}
