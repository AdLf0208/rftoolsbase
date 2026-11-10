// Their bodies (API 8).
fn perform_command(
    game: &mut crate::game::Game<'_>,
    player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    crate::game::rftoolsbase_state::on_perform_command(game, player, command, ran);
}
