fn powah_drops(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: BlockStateId, explosion: Option<f32>, drops: &mut Vec<ItemStack>) {
    let block = game.res.facts().get(state).name.to_string();
    crate::game::powah::block_drops(game, pos, &block, explosion.is_some(), drops);
}
