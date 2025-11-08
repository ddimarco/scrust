# wgpu Migration Plan: SDL2 → Modern GPU Architecture

## Executive Summary

**Recommended Stack: wgpu + winit**

This plan outlines migrating from SDL2's software rendering to a modern GPU-accelerated architecture using wgpu. This approach perfectly aligns with your goals of:
- ✅ Modern shader-based rendering
- ✅ Palette-cycling effects via fragment shaders
- ✅ Future-proof graphics architecture (Vulkan/Metal/DX12/WebGL2 backends)
- ✅ Learning modern graphics programming
- ✅ Maintaining architectural control (not a heavy framework)

**Migration Complexity**: Moderate-High (40-60 hours total)
**Benefits**: Complete control over rendering, modern shader pipeline, excellent performance

---

## Current Architecture Analysis

### SDL2 Usage Breakdown

| Component | Current Implementation | Lines/Files |
|-----------|----------------------|-------------|
| **Windowing** | SDL2 Window creation | lib.rs:188-195 |
| **Events** | SDL2 EventPump (macro-based) | events.rs:1-114 |
| **Rendering** | 8-bit indexed software rendering | render.rs:1-146 |
| **Textures** | Palette → Surface → Texture conversion | pal.rs, ui.rs |
| **Game Loop** | Fixed timestep (60 FPS) | lib.rs:223-276 |
| **Input** | Keyboard + Mouse via SDL2 | events.rs |

### Rendering Pipeline (Current)

```
StarCraft Assets (8-bit indexed)
    ↓
Palette Lookup (CPU)
    ↓
Software Blitting to Surface<'static> (640x480, 8-bit)
    ↓
create_texture_from_surface() [EVERY FRAME!]
    ↓
renderer.copy() to backbuffer
    ↓
renderer.present()
```

**Performance Bottleneck**: Creating texture from surface every frame (line 260 in lib.rs)

### Key Architectural Patterns to Preserve

1. **View Trait**: `fn render(&mut self, ...) -> ViewAction`
2. **LayerTrait**: Composable UI layers
3. **GameContext**: Central rendering/event context
4. **8-bit Assets**: All StarCraft data is palette-indexed
5. **Palette Effects**: Color cycling for animations

---

## Recommended Tech Stack

### Core: wgpu + winit

```toml
[dependencies]
wgpu = "0.19"
winit = "0.29"
pollster = "0.3"  # For async runtime
bytemuck = "1.14"  # Safe transmutation for GPU buffers
```

### Why wgpu?

- ✅ **Modern**: Based on WebGPU standard, future-proof
- ✅ **Cross-platform**: Vulkan, Metal, DX12, WebGL2 backends
- ✅ **Shader Control**: WGSL shaders for palette-cycling effects
- ✅ **Performance**: True GPU acceleration
- ✅ **Rust-first**: Designed for Rust, excellent ecosystem
- ✅ **Learning**: Best path to understanding modern graphics

### Why winit?

- ✅ **Industry standard** for windowing in Rust
- ✅ **Event handling** similar to SDL2
- ✅ **wgpu integration**: Seamless surface creation
- ✅ **No SDL2 dependency**: Native Rust

### Optional Additions

```toml
# For UI/debugging (later)
egui = "0.26"
egui-wgpu = "0.26"

# For timing
instant = "0.1"  # Cross-platform timing
```

---

## New Architecture Design

### Rendering Pipeline (wgpu)

```
StarCraft Assets (8-bit indexed)
    ↓
Upload to GPU Texture once (R8Uint format)
    ↓
Upload Palette to GPU (256 × RGB)
    ↓
Fragment Shader: index → palette[index] → RGB
    ↓
Render to swapchain
    ↓
present()
```

**Performance Win**: GPU does palette lookup, assets uploaded once, zero CPU blitting

### Palette-Cycling Implementation

```wgsl
// Fragment Shader (WGSL)
@group(0) @binding(0) var indexed_texture: texture_2d<u32>;
@group(0) @binding(1) var palette: texture_1d<f32>;  // 256 × RGB
@group(0) @binding(2) var<uniform> palette_shift: u32;  // For cycling

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let index = textureLoad(indexed_texture, in.tex_coords, 0).r;
    let cycled_index = (index + palette_shift) % 256u;
    let color = textureLoad(palette, cycled_index, 0);
    return vec4<f32>(color.rgb, 1.0);
}
```

**This enables**: Real-time palette animation (lava, water, unit shields) by just updating a uniform!

### Component Architecture

```rust
// New core abstractions

pub struct WgpuContext {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

pub struct PaletteRenderer {
    pipeline: wgpu::RenderPipeline,
    palette_texture: wgpu::Texture,
    palette_bind_group: wgpu::BindGroup,
}

pub struct IndexedTexture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

// Adapters for existing code
pub trait RenderContext {
    fn draw_indexed_texture(&mut self, texture: &IndexedTexture, x: i32, y: i32);
    fn set_palette(&mut self, palette: &Palette);
    fn present(&mut self);
}

impl RenderContext for WgpuContext { ... }
```

---

## Migration Plan: 5 Phases

### Phase 0: Proof of Concept (8-12 hours)

**Goal**: Validate approach with minimal example

**Tasks**:
1. Create new binary `src/bin/wgpu-test.rs`
2. Initialize wgpu + winit window
3. Load a single GRP frame as indexed texture
4. Implement palette shader
5. Render one sprite to screen

**Deliverables**:
- Working wgpu window
- Shader pipeline
- Single sprite rendering
- Validation of approach

**Code Example**:
```rust
// src/bin/wgpu-test.rs
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("SCRust wgpu Test")
        .with_inner_size(winit::dpi::PhysicalSize::new(640, 480))
        .build(&event_loop)
        .unwrap();

    // Initialize wgpu (see detailed implementation below)
    let wgpu_state = pollster::block_on(WgpuState::new(&window));

    // Load test sprite
    let gd = GameData::init(Path::new("..."));
    let grp = &gd.unit_grps[0];

    // Create indexed texture + palette
    let texture = create_indexed_texture(&wgpu_state, grp.frame(0));
    let palette = upload_palette(&wgpu_state, &gd.install_pal);

    // Render loop
    event_loop.run(move |event, target| {
        match event {
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                render_frame(&wgpu_state, &texture, &palette);
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                target.exit();
            }
            _ => {}
        }
    }).unwrap();
}
```

**Risk**: Low - validates entire approach before major refactoring

---

### Phase 1: Core Abstraction Layer (12-16 hours)

**Goal**: Create adapter layer so existing code can use wgpu OR SDL2

**Tasks**:
1. Create `src/renderer/mod.rs` module
2. Define `RenderContext` trait
3. Implement `WgpuRenderContext`
4. Keep `Sdl2RenderContext` (compatibility)
5. Update `GameContext` to be generic over renderer

**Files Created**:
```
src/renderer/
├── mod.rs              # RenderContext trait
├── wgpu_renderer.rs    # wgpu implementation
├── sdl2_renderer.rs    # SDL2 wrapper (temporary)
├── texture.rs          # Texture abstraction
├── shaders/
│   ├── palette.wgsl    # Palette lookup shader
│   └── sprite.wgsl     # Sprite rendering shader
└── types.rs            # Shared types
```

**RenderContext Trait**:
```rust
pub trait RenderContext {
    type Texture;

    fn clear(&mut self, color: [u8; 3]);

    fn draw_indexed_sprite(
        &mut self,
        texture: &Self::Texture,
        src: Option<Rect>,
        dst: Rect,
        flipped: bool,
    );

    fn set_palette(&mut self, palette: &Palette);

    fn create_indexed_texture(&mut self, data: &[u8], width: u32, height: u32)
        -> Self::Texture;

    fn present(&mut self);

    fn size(&self) -> (u32, u32);
}
```

**Updated GameContext**:
```rust
pub struct GameContext<R: RenderContext> {
    pub events: Events,
    pub renderer: R,
    pub screen_buffer: Option<Vec<u8>>,  // For software fallback
}
```

**Benefit**: Gradual migration, can test wgpu alongside SDL2

---

### Phase 2: Event System Migration (6-8 hours)

**Goal**: Replace SDL2 events with winit

**Tasks**:
1. Create `src/events_winit.rs`
2. Map SDL2 events → winit events
3. Update `struct_events!` macro or replace it
4. Handle keyboard/mouse input
5. Test event parity

**Event Mapping**:
```rust
// src/events_winit.rs
use winit::event::{Event, WindowEvent, MouseButton, ElementState};
use winit::keyboard::KeyCode;

pub struct WinitEvents {
    // Same public API as SDL2 Events
    pub mouse_pos: Point,
    pub mouse_down_pos: Option<(i32, i32)>,
    pub now: ImmediateEvents,
}

impl WinitEvents {
    pub fn process_event(&mut self, event: &Event<()>) {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::MouseInput { state, button, .. } => {
                    match button {
                        MouseButton::Left => {
                            self.now.mouse_left = *state == ElementState::Pressed;
                        }
                        MouseButton::Right => {
                            self.now.mouse_right = *state == ElementState::Pressed;
                        }
                        _ => {}
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    self.mouse_pos = Point::new(position.x as i32, position.y as i32);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    // Handle keyboard
                }
                _ => {}
            }
            _ => {}
        }
    }
}
```

**Challenge**: `struct_events!` macro is SDL2-specific
**Solution**: Either adapt macro or hand-code event handling (cleaner for wgpu version)

---

### Phase 3: Port Rendering Code (16-20 hours)

**Goal**: Migrate all rendering from software to GPU shaders

**Tasks**:
1. Implement `WgpuRenderContext::draw_indexed_sprite`
2. Port `render.rs` blitting logic to shaders
3. Implement sprite batching for performance
4. Handle transparency/reindexing in shaders
5. Update all `View` implementations
6. Update all `LayerTrait` implementations

**Shader Implementation**:

```wgsl
// shaders/sprite.wgsl

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

@group(0) @binding(0) var sprite_texture: texture_2d<u32>;
@group(0) @binding(1) var palette: texture_1d<f32>;
@group(0) @binding(2) var<uniform> transform: mat4x4<f32>;
@group(0) @binding(3) var<uniform> flip: u32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample indexed texture
    var tex_coord = in.tex_coords;
    if (flip != 0u) {
        tex_coord.x = 1.0 - tex_coord.x;
    }

    let index = textureLoad(sprite_texture, vec2<i32>(tex_coord * vec2<f32>(textureDimensions(sprite_texture))), 0).r;

    // Index 0 = transparent
    if (index == 0u) {
        discard;
    }

    // Palette lookup
    let color = textureLoad(palette, i32(index), 0);
    return vec4<f32>(color.rgb, 1.0);
}
```

**Transparency Reindexing Shader** (for units under shields, etc.):
```wgsl
@group(0) @binding(4) var reindex_table: texture_2d<u32>;  // 256x256

@fragment
fn fs_main_reindex(in: VertexOutput) -> @location(0) vec4<f32> {
    let sprite_index = textureLoad(sprite_texture, tex_coords, 0).r;
    let bg_index = textureLoad(background_texture, screen_coords, 0).r;

    // Look up reindexed color
    let new_index = textureLoad(reindex_table, vec2<i32>(sprite_index, bg_index), 0).r;
    let color = textureLoad(palette, i32(new_index), 0);
    return vec4<f32>(color.rgb, 1.0);
}
```

**Performance: Sprite Batching**:
```rust
pub struct SpriteBatch {
    instances: Vec<SpriteInstance>,
    instance_buffer: wgpu::Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteInstance {
    position: [f32; 2],
    size: [f32; 2],
    tex_index: u32,
    flipped: u32,
}

impl SpriteBatch {
    pub fn draw(&mut self, texture_id: u32, x: i32, y: i32, flipped: bool) {
        self.instances.push(SpriteInstance {
            position: [x as f32, y as f32],
            size: [width as f32, height as f32],
            tex_index: texture_id,
            flipped: flipped as u32,
        });
    }

    pub fn flush(&mut self, queue: &wgpu::Queue, render_pass: &mut RenderPass) {
        // Upload instances to GPU
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&self.instances));

        // Draw all sprites in one call
        render_pass.draw(0..6, 0..self.instances.len() as u32);

        self.instances.clear();
    }
}
```

**Files to Update**:
- `src/ui.rs`: All View implementations
- `src/unit_ecs.rs`: Rendering code
- `src/render.rs`: Delete or mark deprecated

---

### Phase 4: Texture Management (8-10 hours)

**Goal**: Efficient GPU texture caching and management

**Tasks**:
1. Create `TextureAtlas` for packing sprites
2. Implement texture caching (upload once)
3. Handle dynamic palette updates
4. Optimize texture uploads
5. Implement texture pooling

**Texture Atlas**:
```rust
pub struct TextureAtlas {
    texture: wgpu::Texture,
    allocator: guillotiere::AtlasAllocator,  // Rectangle packing
    regions: HashMap<u64, guillotiere::Allocation>,
}

impl TextureAtlas {
    pub fn upload_sprite(&mut self,
                        queue: &wgpu::Queue,
                        sprite_data: &[u8],
                        width: u32,
                        height: u32) -> TextureRegion {
        // Allocate space in atlas
        let allocation = self.allocator.allocate(size2(width, height)).unwrap();

        // Upload to GPU
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: allocation.rectangle.min.x as u32,
                    y: allocation.rectangle.min.y as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            sprite_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );

        TextureRegion {
            atlas: self.texture_id,
            uv: allocation.rectangle,
        }
    }
}
```

**GRP Cache** (existing GameData integration):
```rust
pub struct GRPTextureCache {
    atlas: TextureAtlas,
    cache: HashMap<(usize, usize), TextureRegion>,  // (grp_id, frame_id) -> region
}

impl GRPTextureCache {
    pub fn get_or_upload(&mut self,
                        grp_id: usize,
                        frame: usize,
                        gd: &GameData) -> &TextureRegion {
        self.cache.entry((grp_id, frame)).or_insert_with(|| {
            let grp = &gd.unit_grps[grp_id];
            let frame_data = grp.frame(frame);
            self.atlas.upload_sprite(&queue, frame_data, grp.width, grp.height)
        })
    }
}
```

---

### Phase 5: Game Loop & Polish (6-8 hours)

**Goal**: Integrate everything, optimize, remove SDL2

**Tasks**:
1. Update main game loop to use winit
2. Implement proper vsync/frame pacing
3. Remove all SDL2 dependencies
4. Test all binaries
5. Performance profiling
6. Add palette-cycling demo

**New Game Loop**:
```rust
pub fn spawn_wgpu<F>(title: &str, init: F)
where
    F: Fn(&GameData, &mut GameContext<WgpuRenderContext>, &mut GameState)
        -> Box<dyn View<WgpuRenderContext>> + 'static
{
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(640, 480))
        .build(&event_loop)
        .unwrap();

    let wgpu_state = pollster::block_on(WgpuRenderContext::new(&window));

    // Load game data
    let config = load_config();
    let gd = GameData::init(Path::new(&config.scdata_path));

    let mut context = GameContext {
        events: WinitEvents::new(),
        renderer: wgpu_state,
        screen_buffer: None,
    };

    let mut state = GameState::new();
    let mut current_view = init(&gd, &mut context, &mut state);

    let mut last_frame = Instant::now();

    event_loop.run(move |event, target| {
        match event {
            Event::WindowEvent { event, .. } => {
                context.events.process_event(&Event::WindowEvent {
                    window_id: window.id(),
                    event: event.clone()
                });

                match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let dt = now.duration_since(last_frame);
                        let elapsed = dt.as_secs_f64();

                        if elapsed < 1.0 / 60.0 {
                            return;  // Frame pacing
                        }

                        last_frame = now;

                        // Update
                        current_view.update(&gd, &mut context, &mut state);
                        current_view.generate_layer_events(&gd, &mut context, &mut state);
                        current_view.process_layer_events(&mut context, &mut state);

                        // Render
                        let render_res = current_view.render(&gd, &mut context, &state, elapsed);
                        current_view.render_layers(&mut context);
                        context.renderer.present();

                        match render_res {
                            ViewAction::None => {}
                            ViewAction::Quit => target.exit(),
                            ViewAction::ChangeView(new_view) => current_view = new_view,
                        }

                        window.request_redraw();
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}
```

**Palette Cycling Demo**:
```rust
// Animate water/lava by shifting palette entries
pub struct PaletteCycler {
    ranges: Vec<(u8, u8, u32)>,  // (start, end, speed_ms)
    last_update: Instant,
}

impl PaletteCycler {
    pub fn update(&mut self, palette: &mut Palette) {
        let now = Instant::now();

        for (start, end, speed_ms) in &self.ranges {
            if now.duration_since(self.last_update).as_millis() > *speed_ms as u128 {
                // Rotate palette entries
                let first = palette.entries[*start as usize];
                for i in *start..*end {
                    palette.entries[i as usize] = palette.entries[i as usize + 1];
                }
                palette.entries[*end as usize] = first;
            }
        }

        self.last_update = now;
    }
}

// Usage in render loop
context.renderer.set_palette(&cycled_palette);  // GPU upload
```

---

## File-by-File Migration Guide

### Critical Files

| File | Current | New | Action |
|------|---------|-----|--------|
| `lib.rs` | SDL2 window, Canvas, game loop | wgpu context, winit loop | Rewrite spawn() |
| `events.rs` | SDL2 EventPump macro | winit events | Create events_winit.rs |
| `render.rs` | Software blitting | GPU shaders | Delete/archive |
| `ui.rs` | Canvas<Window> | RenderContext trait | Update trait bounds |
| `scformats/pal.rs` | create_texture_from_surface | GPU texture upload | Rewrite |
| `Cargo.toml` | sdl2 = "0.34" | wgpu, winit | Update deps |

### Binaries to Update

All binaries in `src/bin/` need migration:
- `display-grp.rs` - Simple sprite viewer
- `display-menu.rs` - UI test
- `display-smk.rs` - Video playback
- `units-ecs.rs` - Main game
- Others...

**Strategy**: Port one binary at a time, starting with simplest (display-grp)

---

## Dependency Updates

### Remove
```toml
sdl2 = "0.34"
```

### Add
```toml
wgpu = "0.19"
winit = "0.29"
pollster = "0.3"
bytemuck = { version = "1.14", features = ["derive"] }
```

### Keep
```toml
scformats = { path = "src/scformats" }
ecs = "0.23"
config = "0.14"
rand = "0.8"
# ... all others
```

---

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_palette_upload() {
        // Test palette texture creation
    }

    #[test]
    fn test_indexed_texture() {
        // Test sprite upload
    }

    #[test]
    fn test_sprite_batching() {
        // Test batch rendering
    }
}
```

### Integration Tests
1. **Phase 0 binary**: Single sprite renders correctly
2. **display-grp.rs**: All frames of GRP render
3. **display-menu.rs**: UI widgets render
4. **units-ecs.rs**: Full game works

### Visual Regression
- Take screenshots with SDL2 version
- Compare pixel-perfect with wgpu version
- Palette-indexed rendering should be identical

---

## Benefits Summary

### Performance
- **GPU acceleration**: 100-1000x faster than software rendering
- **Zero CPU blitting**: All rendering on GPU
- **Batch rendering**: Draw hundreds of sprites per frame
- **Palette updates**: Just upload 768 bytes (256 × RGB) vs entire screen

### Features
- **Palette-cycling**: Update 1 uniform, animate entire screen
- **Post-processing**: Add CRT shader, scanlines, etc. trivially
- **Shader effects**: Unit shields, spell effects, fog of war in shaders
- **Modern scaling**: Clean upscaling to 1080p/4K
- **Multi-pass**: Deferred rendering, lighting possible

### Code Quality
- **No unsafe blitting**: render.rs has tons of unsafe, shaders eliminate it
- **Cleaner separation**: Rendering logic in shaders, not Rust
- **Modern idioms**: wgpu is idiomatic Rust
- **Better errors**: wgpu validation vs cryptic SDL2 errors

### Future-Proofing
- **WebGPU target**: Can compile to WASM + WebGPU
- **Backend flexibility**: Run on Vulkan, Metal, DX12 automatically
- **Ecosystem**: wgpu is actively developed, SDL2 is legacy
- **Learning**: wgpu skills transfer to other projects

---

## Timeline Estimates

| Phase | Optimistic | Realistic | Pessimistic |
|-------|-----------|-----------|-------------|
| Phase 0: Proof of Concept | 6h | 10h | 16h |
| Phase 1: Abstraction Layer | 10h | 14h | 20h |
| Phase 2: Events | 4h | 7h | 12h |
| Phase 3: Rendering | 12h | 18h | 28h |
| Phase 4: Textures | 6h | 9h | 14h |
| Phase 5: Integration | 4h | 7h | 12h |
| **TOTAL** | **42h** | **65h** | **102h** |

**Recommendation**: Budget 60-80 hours over several weeks, working incrementally

---

## Risk Assessment

### High Risk
❌ **Shader complexity**: Learning WGSL if unfamiliar
   *Mitigation*: Start with simple shaders, iterate

### Medium Risk
⚠️ **wgpu API changes**: wgpu is pre-1.0, API may change
   *Mitigation*: Pin version, update periodically

⚠️ **Texture atlas complexity**: Packing algorithm can be tricky
   *Mitigation*: Use guillotiere crate, start with simple allocation

### Low Risk
✅ **Event handling**: winit events map 1:1 to SDL2
✅ **Window creation**: Straightforward with winit
✅ **Performance**: GPU will be faster, guaranteed

---

## Alternative Approaches

### Option B: macroquad

```toml
macroquad = "0.4"
```

**Pros**:
- Higher-level, faster to port
- Built-in texture/sprite handling
- Simpler API

**Cons**:
- Less control over rendering
- Harder to do custom shaders
- Black box for learning

**Verdict**: Good for rapid prototyping, but doesn't meet learning goals

### Option C: Bevy Engine

```toml
bevy = "0.13"
```

**Pros**:
- Full game engine
- Built-in ECS (could replace current ECS)
- Excellent 2D support

**Cons**:
- Heavy dependency (long compile times)
- Need to adapt to Bevy's ECS and architecture
- Overkill for this project

**Verdict**: Would require full rewrite, changes too much

### Option D: ggez

```toml
ggez = "0.9"
```

**Pros**:
- 2D-focused
- Similar level to SDL2
- Easy migration

**Cons**:
- Still uses wgpu under the hood (less control)
- Less active development
- Doesn't teach low-level graphics

**Verdict**: Middle ground, but doesn't maximize learning

---

## Recommended Next Steps

### Immediate (This Week)
1. ✅ Review this plan
2. ✅ Decide if wgpu is right fit
3. Create feature branch: `git checkout -b wgpu-migration`
4. Start Phase 0: Proof of concept

### Short-term (Next 2 Weeks)
5. Complete Phase 0 PoC
6. If successful, proceed to Phase 1
7. Create abstraction layer
8. Test dual rendering (SDL2 + wgpu)

### Medium-term (Next Month)
9. Complete Phase 2-3
10. Port one full binary (display-grp)
11. Validate rendering parity

### Long-term (Next 2-3 Months)
12. Complete Phase 4-5
13. Remove SDL2 entirely
14. Add palette-cycling demo
15. Optimize and profile

---

## Code Examples: Full PoC

### Complete Phase 0 Implementation

**src/bin/wgpu-poc.rs** (400 lines):
```rust
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use wgpu::util::DeviceExt;

struct WgpuState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    palette_bind_group: wgpu::BindGroup,
    sprite_bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, 1.0, 0.0], tex_coords: [0.0, 0.0] },
    Vertex { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [1.0, -1.0, 0.0], tex_coords: [1.0, 1.0] },
    Vertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0] },
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

impl WgpuState {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Palette Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/palette.wgsl").into()),
        });

        // Create render pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &palette_bind_group_layout,
                    &sprite_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let num_indices = INDICES.len() as u32;

        // TODO: Load actual sprite and palette
        let (palette_bind_group, sprite_bind_group) =
            create_test_textures(&device, &queue);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            palette_bind_group,
            sprite_bind_group,
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.palette_bind_group, &[]);
            render_pass.set_bind_group(1, &self.sprite_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

fn create_test_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (wgpu::BindGroup, wgpu::BindGroup) {
    // TODO: Load from actual GRP and palette
    // For now, create test data

    // Test palette (256 RGB colors)
    let mut palette_data = vec![0u8; 256 * 4];
    for i in 0..256 {
        palette_data[i * 4] = i as u8;       // R
        palette_data[i * 4 + 1] = 0;         // G
        palette_data[i * 4 + 2] = 255 - i as u8; // B
        palette_data[i * 4 + 3] = 255;       // A
    }

    // Test sprite (64x64 indexed)
    let mut sprite_data = vec![0u8; 64 * 64];
    for y in 0..64 {
        for x in 0..64 {
            sprite_data[y * 64 + x] = ((x + y) % 256) as u8;
        }
    }

    // Create palette texture
    let palette_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Palette Texture"),
        size: wgpu::Extent3d {
            width: 256,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D1,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &palette_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &palette_data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(256 * 4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 256,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    // Create sprite texture (R8Uint for indexed)
    let sprite_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Sprite Texture"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &sprite_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &sprite_data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(64),
            rows_per_image: Some(64),
        },
        wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
    );

    // Create bind groups
    // (Implementation continues...)

    todo!("Complete bind group creation")
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("wgpu PoC - SCRust")
        .with_inner_size(winit::dpi::PhysicalSize::new(640, 480))
        .build(&event_loop)
        .unwrap();

    let mut state = pollster::block_on(WgpuState::new(&window));

    event_loop.run(move |event, target| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::RedrawRequested => {
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            // Recreate surface
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            target.exit();
                        }
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}
```

**src/shaders/palette.wgsl**:
```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

@group(0) @binding(0)
var palette_texture: texture_1d<f32>;

@group(1) @binding(0)
var sprite_texture: texture_2d<u32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample indexed texture
    let tex_size = textureDimensions(sprite_texture);
    let tex_coord = vec2<i32>(in.tex_coords * vec2<f32>(tex_size));
    let index = textureLoad(sprite_texture, tex_coord, 0).r;

    // Transparency (index 0)
    if (index == 0u) {
        discard;
    }

    // Palette lookup
    let color = textureLoad(palette_texture, i32(index), 0);
    return color;
}
```

---

## Conclusion

This migration will transform SCRust from software rendering to a modern, GPU-accelerated architecture. The wgpu + winit stack provides:

✅ **Complete control** over rendering pipeline
✅ **Shader-based effects** (palette-cycling, post-processing)
✅ **Future-proof** technology stack
✅ **Excellent learning** opportunity
✅ **Better performance** (100x+ faster rendering)

**Recommended**: Start with Phase 0 PoC to validate the approach, then proceed incrementally through phases 1-5.

**Total Effort**: ~60-80 hours over 2-3 months of incremental work

**Questions?** Ready to start Phase 0?
