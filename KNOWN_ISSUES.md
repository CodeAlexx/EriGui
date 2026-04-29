# Known issues

## TODO: rebuild FileManager widget with real filesystem (2026-04-28)

**Status**: open. Original `file_manager.rs` was deleted in wave-1
commit `74659fb` because it was a non-functional stub: every method
that should have read the filesystem returned hardcoded mock data
("Home / Documents / Downloads / Pictures" tree, "document.txt /
image.png / Projects" list). `refresh()` was empty. `draw()` did
nothing. `handle_event` returned Ignored. Pure layout shell.

**What to build**: a real `FileManager` widget composed of
- A left `TreeView` "sidebar" listing reachable bookmarks (Home,
  Documents, /tmp, etc.) plus the current path's parent chain
- A right `ListView` showing the current directory's entries via
  `std::fs::read_dir`
- A top `Breadcrumb` showing the current path
- Optional: hidden-files toggle, sort dropdown, search box

**Estimate**: 2-3 hours. Watch for path-traversal pitfalls
(canonicalize paths, never trust user-typed paths to stay inside a
sandbox if one is configured).

**Note**: this is independent of `file_dialog.rs` (the in-app open/
save dialog, which is now a fallback since `tinyfiledialogs` is the
live picker per wave-1 commit `6f74592`).

---

## inference-flame API drift — erigui-nodes won't compile (2026-04-28)

**Status**: open. Discovered during the wave-2 Rustification pass.

**Symptom**: `cargo build --workspace` (or any test/run that touches
`erigui-nodes` and its dependents `erigui-runtime`, `erigui-workflow`,
`erigui-app`, `erigui-examples`) fails with three errors:

```
error[E0432]: unresolved import `inference_flame::sampling::klein_sampling::box_muller_noise`
   --> erigui-nodes/src/builtin/k_sampler.rs:49
error[E0432]: unresolved import `inference_flame::lora`
   --> erigui-nodes/src/builtin/load_lora.rs:37
error[E0599]: no method named `clone` found for struct `KleinTransformer`
   --> erigui-nodes/src/builtin/{k_sampler.rs,load_lora.rs,load_checkpoint.rs}
```

**Cause**: `erigui-nodes` has a path dependency on `inference-flame`
in the EriDiffusion repo (`/home/alex/EriDiffusion/inference-flame`).
That crate is in the middle of a TensorIterator port (handoff at
`/home/alex/EriDiffusion/HANDOFF_2026-04-22_TENSORITERATOR_PORT.md`)
and has reshaped/removed several public symbols `erigui-nodes`
imports.

**Verified scope**: `erigui-core`, `erigui-widgets`, and
`erigui-rendering` are **not affected** — they have no flame
dependency. Their tests run clean (376 passed / 0 failed / 2 ignored
as of commit `bc6fbac`).

**Specific call-sites needing repair in erigui-nodes**:

| File:line | Broken import / call | Likely upstream change |
|-----------|---------------------|------------------------|
| `erigui-nodes/src/builtin/k_sampler.rs:49` | `box_muller_noise` | renamed / moved within `klein_sampling` module |
| `erigui-nodes/src/builtin/load_lora.rs:37` | `inference_flame::lora` (the module itself) | LoRA module restructured under flame-diffusion or moved |
| `erigui-nodes/src/builtin/k_sampler.rs:48` `load_lora.rs:38` `load_checkpoint.rs:34` | `KleinTransformer::clone` | `Clone` impl removed from `KleinTransformer` (likely intentional — the model holds device state) |

Plus these still-importable but worth-double-checking sites:
- `erigui-nodes/src/handles.rs:14-15` — `Qwen3Encoder`, `KleinVaeDecoder`, `KleinVaeEncoder`
- `erigui-nodes/src/builtin/load_checkpoint.rs:35-36`
- `erigui-nodes/src/bin/smoke_vae_decode.rs:35`

**Fix approach** (out of scope for the current Rustification pass —
file an issue on github.com/CodeAlexx/EriGui):

1. Read the current `inference-flame` API surface (after the
   TensorIterator port). Particularly `sampling/klein_sampling.rs`,
   the LoRA module structure (might be under `lora/` directory now or
   merged into something else), and `models/klein.rs`.
2. For `box_muller_noise`: it likely either renamed (e.g.,
   `gaussian_noise`) or was inlined into the caller. Find the new
   noise-generation entry point and update the import.
3. For `inference_flame::lora`: the LoRA infrastructure may have
   moved to `flame-diffusion` (the trainer-side) and inference-flame
   may now consume a smaller surface. Find the right path.
4. For `KleinTransformer::clone`: the model probably no longer impls
   `Clone` because clones of model state are expensive / undefined
   when device-resident weights are involved. Replace `.clone()` call
   sites with `Arc<KleinTransformer>` sharing or with a deliberate
   `KleinTransformer::reload(...)` call.

**Estimated effort**: 1-3 hours to repair erigui-nodes against
current inference-flame, depending on how invasive the API changes
are.

**Until then**: the widget library Rustification effort can continue
without touching this. The visual Klein-inference demo path is
inaccessible until erigui-nodes is restored.
