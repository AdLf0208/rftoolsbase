// `/rftoolsbase info|energy|infused|held` (API 8).
fn register_commands(commands: &mut Vec<NativeCommand>) {
    crate::game::rftoolsbase_state::on_register_commands(commands);
}
