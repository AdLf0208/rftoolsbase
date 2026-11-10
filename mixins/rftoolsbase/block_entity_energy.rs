// The 60,000 FE insert-only buffer (API 142).
fn declare_block_entity_energy(
    block: &str,
    energy: &mut Option<horizon_client::containers::ModEnergyBuffer>,
) {
    crate::game::rftoolsbase_state::on_block_entity_energy(block, energy);
}
