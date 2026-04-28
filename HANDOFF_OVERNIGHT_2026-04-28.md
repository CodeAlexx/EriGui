# Overnight handoff — 2026-04-28

User said: "i need to sleep, check other primitives to see working, if broke fix. we refactored for rust only now so many bugs un covered."

Worked from ~04:30 to ~05:15 PDT. 8 commits pushed to master. All workspace tests pass (293+ across the workspace, 48 widget_tests).

## What got fixed (in commit order)

| Commit | Fix |
|---|---|
| `46bdf19` | node_graph routes Text-field events through library `TextInput` widget. Kills duplicate `FieldEditState` editor. Adds `Field.name` (programmatic key, distinct from `label`) so executor's per-node fields HashMap matches what node bodies look up. |
| `1360c36` | combo_box, spin_box, color_picker, date_time_picker now handle `Event::TextInput` (production winit 0.29 keyboard path). Previously only handled `Key::Character` legacy form which never fires in real GUIs — typing into these widgets did literally nothing. |
| `26a6f5c` | text_input, text_area, node_graph: Ctrl-shortcut detection now requires `CTRL && !ALT`. Linux AltGr emits Ctrl+Alt; without this fix, AltGr+letter sequences (typing @, €, etc.) got eaten as Ctrl-shortcuts. |
| `387c568` | Slider + Checkbox: focus state was hardcoded to false / set_focused was a no-op (focus was a permanent lie). Fixed. Plus added keyboard support: arrows/Home/End/PageUp/PageDn for Slider, Space/Enter for Checkbox. |
| `501869d` | ListView gets full keyboard nav: Up/Down/Home/End/PageUp/PageDn when focused. |
| `715f0de` | ListView gains scroll: mouse wheel (3 items/tick), auto-scroll-to-selected on keyboard nav past viewport, click hit-test correctly accounts for scroll. |
| `1f9dea8` | Button: Space/Enter on focused button fires on_click (standard accessibility). Dialog: Escape closes (also standard). |
| `42425e8` | ListView draws a visible scrollbar (8px track, proportional thumb) when content overflows viewport. |

## Tests added

48 widget_tests now (was ~30 pre-overnight). New ones:
- 2 ComboBox (TextInput consumed when open, ignored when closed)
- 3 Slider (focus round-trip, kbd nav, ignored when unfocused)
- 3 Checkbox (focus round-trip, Space toggles, ignored when unfocused)
- 5 ListView keyboard (down/up clamping, Home/End, unfocused)
- 3 ListView scroll (wheel, clamped, auto-scroll-to-selected)
- 2 Button (kbd activation, ignored when unfocused)

Plus existing: 27 lib + 74 bug_fix + 5 progress = 154 widget tests total. All pass.

## What you wanted that I did NOT do

- **Font scaling for HiDPI/4K**: still not done. Theme stores font sizes in raw px; on 4K the rendered glyphs are half the size users expect. Real fix is threading `scale_factor` through the draw path. Couldn't do without GPU to test.
- **Native file picker without GTK**: still using the in-app FileDialog widget. `tinyfiledialogs` (no GTK link) would be drop-in but I didn't pull it overnight to avoid surprising you with a new dep.
- **Drag-the-scrollbar-thumb to scroll**: scrollbar is visual only, drag-to-scroll is TODO.
- **TabControl, RadioButton, Accordion keyboard nav**: skipped — RadioButton uses a global mutex-managed group manager that complicates things; TabControl needs `can_focus = true` change which is API breaking; Accordion would need per-panel focus tracking.

## What's likely still broken (audit hints)

- `scroll_view.rs` is a stub — placeholders like "In a real implementation, we'd need access to the widget manager here". Not load-bearing (only `widget_gallery.rs` uses it).
- `file_dialog` still uses the FileDialog widget (the user-flagged broken one). The ListView fix flows in for the file list, but the dialog's overall layout, fonts, and other UI may still be unsatisfying.
- `dock_panel` has TODOs.
- HiDPI broken across the board.

## How to test

```bash
cd /home/alex/EriGui/rust-gui
LD_LIBRARY_PATH="/home/alex/libs/libtorch/lib:$LD_LIBRARY_PATH" \
  cargo run --release -p erigui-app
```

Try: click a Text field in a node, click in the MIDDLE of the text — cursor should land where you clicked (was: end). Type. Use arrow keys, Home/End. Ctrl+A to select all. The library `TextInput` is now the editor, so any TextInput improvement future-helps node-graph fields too.

Try: long file lists in the file dialog should now scroll with the wheel and auto-scroll when you arrow-key past the visible band. Visible scrollbar on the right.

## Next-day priorities (my honest read)

1. HiDPI font scaling — biggest visible UX issue you hit twice yesterday.
2. Native file picker — `tinyfiledialogs` is the cleanest no-GTK path.
3. Verify the changes hold up when you actually run the app (I had no GPU to test).
