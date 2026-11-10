// ... and ticks on the server (API 15).
fn declare_block_entity_ticks(block: &str, ticks: &mut bool) {
    crate::game::rftoolsbase_state::on_block_entity_ticks(block, ticks);
}
