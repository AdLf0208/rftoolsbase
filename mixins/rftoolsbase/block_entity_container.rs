// Two slots — shards 0, the machine 1 — and the machine menu (API 16/17).
fn declare_block_entity_container(
    block: &str,
    container: &mut Option<horizon_client::containers::ModContainer>,
) {
    crate::game::rftoolsbase_state::on_container(block, container);
}
