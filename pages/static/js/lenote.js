import init, { run_app } from '/ui/lenote_ui.js';
async function main() {
  await init('/ui/lenote_ui_bg.wasm');
  run_app();
}
main()
