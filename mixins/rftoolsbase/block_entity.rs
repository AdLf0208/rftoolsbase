// The machine gets a block entity (API 15).
fn declare_block_entity(block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
    crate::game::rftoolsbase_state::on_block_entity(block, entity);
}
