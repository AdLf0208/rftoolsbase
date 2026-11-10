// One server tick of the infusion cycle (API 17).
fn observe_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut horizon_client::containers::ModBlockEntity,
) {
    crate::game::rftoolsbase_state::on_tick(game, pos, entity);
}
