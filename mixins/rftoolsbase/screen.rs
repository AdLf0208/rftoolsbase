// The machine menu's two slots and 180 x 152 panel (API 19).
fn declare_screen(block: &str, screen: &mut Option<NativeScreen>) {
    crate::game::rftoolsbase_state::on_screen(block, screen);
}
