fn powah_neighbor(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId) {
    crate::game::powah::neighbor_changed(&level.facts.get(state).name, [pos.x, pos.y, pos.z]);
}
