// An infused item seeds the block it becomes (API 14).
fn observe_place_block(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    pos: [i32; 3],
) {
    crate::game::rftoolsbase_state::on_place_block(game, stack, pos);
}
