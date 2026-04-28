# EriGui Node Graph — Wave 2 Seed Spec (Klein architecture first)

> Status: SEED. Wave 2 builders implement node bodies against this seed and `nodes_spec.md`.
> Scope: Klein architecture ONLY. Other architectures error with `NodeError::Other("not yet implemented for {arch:?}")`.
> Reason: project rule — "do it right first then do faster". One arch end-to-end before generalizing.

## Reference for the Klein pipeline

The end-to-end Klein inference pipeline already works at `/home/alex/EriDiffusion/inference-flame/src/bin/klein_lora_infer.rs`. Read it. Each wave 2 node body is essentially carving one stage out of that binary into a `NodeType::execute()`.

Key calls from that file:
- Prompt encoding: `Qwen3Encoder::new(weights, config, device).encode(&token_ids)` → `Tensor`
- Model load: `NextDiT::new_resident(weights)` (look up the actual constructor)
- LoRA apply: `model.set_lora(Arc::new(lora_stack))` — **post-construction; A1's spec was wrong about needing a new helper**
- Sampling: `klein_sampling::euler_denoise(model_fn, noise, timesteps)`, schedule from `klein_sampling::get_schedule(steps, image_seq_len)`
- VAE decode: `vae.decode(&latents)` → `Tensor` (rgb image)

## Per-node mapping

### `core/load_checkpoint` → `builtin/load_checkpoint.rs`

**Inputs**: none.
**Fields**: `path: FilePath` (extensions = `["safetensors"]`).
**Outputs**: `model: Model { arch: Klein, handle: Arc<NextDiT> }`, `clip: Clip { arch: Klein, handle: Arc<Qwen3EncoderHandle> }`, `vae: Vae { arch: Klein, handle: Arc<KleinVae> }`.

**Implementation strategy**:
1. Klein checkpoints are split: DiT weights, encoder weights, VAE weights — separate files in real use.
2. **Wave 2 simplification**: `path` field points to a JSON sidecar that names all 3 paths, OR field becomes a directory containing them. Pick the JSON sidecar — it matches Comfy's "checkpoint bundle" convention better.
3. Sidecar format (concrete):
   ```json
   {
     "arch": "klein",
     "dit": "/path/to/klein9b.safetensors",
     "encoder": "/path/to/qwen_3_4b.safetensors",
     "vae": "/path/to/klein_vae.safetensors"
   }
   ```
4. Load all three, return three handles.
5. `arch` field in sidecar must equal `"klein"` for wave 2; else return `NodeError::Other("only klein supported in wave 2")`.

**`Qwen3EncoderHandle`** is a small wrapper because the encoder needs `tokenizer` + `weights` together:
```rust
pub struct Qwen3EncoderHandle {
    pub encoder: Qwen3Encoder,
    // Optional: cached tokenizer if Klein uses a fixed one.
}
```
Define this in `erigui-nodes/src/handles.rs`.

### `core/load_lora` → `builtin/load_lora.rs`

**Inputs**: `model: Model`, `clip: Clip`.
**Fields**: `path: FilePath`, `strength: Number{0..2, 0.05}`.
**Outputs**: `model: Model`, `clip: Clip` (modified copies; original Arc-shared so other branches see unmodified).

**Implementation**:
1. Downcast `model.handle` to `Arc<NextDiT>`.
2. Load LoRA weights from `path` via `inference_flame::lora_merge::*` — use the existing converter logic that handles 4 formats (KleinTrainer/AiToolkit/KohyaSdxl/DiffusersKohya/TrainerSplit).
3. Build a `LoraStack` (or use the already-built one).
4. **Cannot mutate the Arc<NextDiT>** because Arc only allows immutable access. Two options:
   - (a) `Arc::make_mut` if refcount == 1 (unlikely if the model output already feeds another branch).
   - (b) Clone the `NextDiT` (deep-clone its internals), apply `set_lora` on the clone, return new Arc.
5. Pick (b). It's the safe path; perf cost is the model deep-clone, but loaders run once at queue-start.
6. CLIP/encoder is passed through unchanged for wave 2 (Klein LoRAs don't touch the encoder; if the LoRA file has CLIP weights, log a warning and ignore for now).
7. Strength: `LoraStack` already stores per-module strength; multiply through before construction.

If the model's `arch` is not Klein, return `NodeError::Other("..."`).

### `core/encode_prompt` → `builtin/encode_prompt.rs`

**Inputs**: `clip: Clip`.
**Fields**: `text: Text`, `negative: Bool`.
**Outputs**: `conditioning: Conditioning`.

**Implementation**:
1. Downcast `clip.handle` to `Arc<Qwen3EncoderHandle>`.
2. Tokenize text. Klein uses chat-template formatting per `klein_lora_infer.rs:194-199`: `format!("{prompt}{trail}")` with the same template. Copy the template logic verbatim.
3. `encoder.encode(&token_ids)` → `Tensor`.
4. Pack into `ConditioningPack { embeds: tensor, pooled: None, mask: <attention mask>, arch: Klein }`. Mask is from tokenizer output.
5. `negative` field: if true, log/tag this as a negative branch (no behavior difference at encode time — same encoder; the K-sampler picks which branch is which).

### `core/k_sampler` → `builtin/k_sampler.rs`

**Inputs**: `model: Model`, `positive: Conditioning`, `negative: Conditioning`, `latent: Latent`.
**Fields**: `seed: Number (default 42, render via FieldKind::Number 0..u32::MAX)`, `steps: Number{1..200}`, `cfg: Number{0..30}`, `shift: Number{0..10}` (already corrected from skeptic's finding).
**Outputs**: `latent: Latent { arch: Klein, tensor: <denoised> }`.

**Implementation**:
1. ArchMismatch checks: all four input arches must equal `Klein`.
2. Downcast `model.handle` to `Arc<NextDiT>`.
3. Build initial noise via `randn_bf16(latent.tensor.shape().dims(), &device)` seeded with the `seed` field — use `flame_core` RNG with seed.
4. Schedule: `klein_sampling::get_schedule(steps, image_seq_len)` where `image_seq_len = (h/8) * (w/8)` derived from latent shape. (`shift` field maps to the empirical-mu calculation; for wave 2 just pass through.)
5. `model_fn` closure: call `model.forward_with_lora(latents, timestep, positive_embeds, attention_mask)` — exact name per the existing klein binary.
6. CFG: standard formula `pred = pred_uncond + cfg * (pred_text - pred_uncond)`. Both branches encoded via positive/negative conditioning.
7. `klein_sampling::euler_denoise(model_fn, noise, &schedule)` returns the denoised latent.
8. Progress: if `progress: Some(sink)`, call `sink.step(i, total, "klein euler")` inside the model_fn each step.
9. Return `Latent { arch: Klein, tensor: denoised }`.

### `core/vae_decode` → `builtin/vae_decode.rs`

**Inputs**: `latent: Latent`, `vae: Vae`.
**Fields**: none.
**Outputs**: `image: Image(<rgb tensor>)`.

**Implementation**:
1. ArchMismatch check: latent.arch == vae.arch == Klein.
2. Downcast `vae.handle` to `Arc<KleinVae>`.
3. `vae.decode(&latent.tensor)` → `Tensor` ([B, 3, H, W] BF16 in [-1, 1] range or [0, 1] — match what klein_lora_infer.rs does).
4. Return `Image(tensor)`. Conversion to displayable image happens in `save_image` or in the wave-3 image preview node.

### `core/save_image` → `builtin/save_image.rs`

**Inputs**: `image: Image`.
**Fields**: `filename_prefix: Text (default "EriGui")`.
**Outputs**: none.

**Implementation**:
1. Convert tensor to u8 image:
   - Transfer to CPU (`tensor.to_cpu()`).
   - Apply scaling: if range is [-1, 1] → `(x + 1) * 127.5`; if [0, 1] → `x * 255`. Pick whichever klein_lora_infer.rs uses.
   - Clamp to [0, 255].
2. Filename: `{prefix}_{timestamp}_{seed_or_hash}.png` — timestamp via `chrono::Local::now().format("%Y%m%d_%H%M%S")`. Increment a small counter if collision.
3. Write to `<output_dir>/`. For wave 2, output dir is hardcoded `./output/` relative to cwd. Wave 3 makes it configurable.
4. Use the `image` crate (already a dep somewhere in inference-flame or its deps).

## Cross-cutting requirements

- **Single CudaDevice**: each node body acquires the global device via `flame_core::CudaDevice::new(0)?` (or whatever the canonical accessor is in inference-flame). Don't create per-node devices.
- **Error wrapping**: `?` on any `flame_core::Error` auto-converts to `NodeError::Inference(..)` via the `From` impl A1 added.
- **Tests**: each builder adds at least one test that exercises the schema (validates ports/fields are present and named correctly). Real model-loading tests would need a multi-GB checkpoint on disk — defer to integration testing.
- **No GPU calls in unit tests**: the test that just validates the `schema()` output is fine without a CUDA device. Don't try to run `execute()` in unit tests.

## What wave 2 does NOT do

- Per-arch dispatch beyond Klein (other arches return error).
- Executor / topo-sort / cache (wave 3).
- Image preview rendering (wave 3).
- Live progress UI side (wave 3).
- The `set_lora` deep-clone perf optimization. Just deep-clone for now.

## Files each builder owns

| Builder | Files written | Files read (no write) |
|---------|--------------|----------------------|
| B1 (load_checkpoint) | `builtin/load_checkpoint.rs`, maybe new `handles.rs` | klein_lora_infer.rs, inference-flame public API |
| B2 (load_lora) | `builtin/load_lora.rs` | inference-flame lora_merge, lora |
| B3 (encode_prompt) | `builtin/encode_prompt.rs` | klein_lora_infer.rs, qwen3_encoder.rs |
| B4 (k_sampler) | `builtin/k_sampler.rs` (already partially correct) | klein_lora_infer.rs, klein_sampling.rs |
| B5 (vae_decode) | `builtin/vae_decode.rs` | inference-flame vae |
| B6 (save_image) | `builtin/save_image.rs` | (no inference-flame; just `image` crate) |

If multiple builders need to extend `handles.rs`, B1 owns it; others coordinate via the seed (e.g. B3 may need `Qwen3EncoderHandle` from B1).

## Coordination points

- B1 defines `Qwen3EncoderHandle` and `KleinVae` wrappers in `handles.rs`. B3 and B5 read those.
- B2 may need to deep-clone NextDiT — that's an inference-flame concern. If `NextDiT` doesn't `derive(Clone)` and there's no good way, B2 falls back to documenting "lora apply mutates the input handle if refcount==1; otherwise returns an error".
- All builders add `inference-flame` as a path dep to `erigui-nodes/Cargo.toml`. First one to land does the Cargo.toml edit; others verify it's there before adding.
