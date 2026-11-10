// The item's "Infused: N%" line (API 12).
fn add_tooltip(
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    lines: &mut Vec<(Text, u32)>,
) {
    crate::game::rftoolsbase_state::on_tooltip(game, stack, lines);
}
