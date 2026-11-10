// Before content loads: registers `rftoolsbase:infused` (API 65).
fn register_component_types(types: &mut Vec<String>) {
    crate::game::rftoolsbase_state::on_component_types(types);
}
