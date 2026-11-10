// The buffer, infused level and countdown readouts (API 19).
fn draw_screen(
    game: &crate::game::Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut horizon_client::hud::HudBatch,
    origin: [f32; 2],
) {
    crate::game::rftoolsbase_state::on_screen_draw(game, block, data, batch, origin);
}
