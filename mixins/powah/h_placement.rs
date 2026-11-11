fn powah_placement(level: &crate::game::GameLevel<'_>, state: horizon_world::registry::BlockStateId, context: &horizon_world::level::PlacementContext, answer: &mut Option<Option<horizon_world::registry::BlockStateId>>) {
    crate::game::powah::state_for_placement(level, state, context, answer);
}
