use saddle_diff_plugin::app::DiffPlugin;
fn main() -> anyhow::Result<()> {
    saddle_plugin_sdk::run(|| Box::<DiffPlugin>::default())
}
