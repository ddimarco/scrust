# Detailed Plan to Fix UI Code

## Problem Summary

The SDL2 0.34 upgrade requires significant refactoring of how textures are created and managed. Additionally, several dependencies (rand, config) have breaking API changes.

---

## Issue Categories

### 1. SDL2 Texture Lifetime Issues (Priority: HIGH)

**Problem**: In SDL2 0.34+, textures must be created from a `TextureCreator` and have explicit lifetimes.

**Current State**:
```rust
// BROKEN - No lifetime parameter
pub struct MousePointer {
    textures: Vec<Vec<Texture>>,  // ❌ Missing lifetime
    // ...
}
```

**Required State**:
```rust
// FIXED - With lifetime parameter
pub struct MousePointer<'tc> {
    textures: Vec<Vec<Texture<'tc>>>,  // ✅ Lifetime tied to TextureCreator
    // ...
}
```

**Affected Files**:
- `src/ui.rs:62` - MousePointer::textures
- `src/ui.rs:132` - MiniMap::minimap
- `src/ui.rs:207` - SelectionPanel::text
- `src/ui.rs:298` - UiLayer::hud_texture

---

### 2. TextureCreator Management (Priority: HIGH)

**Problem**: TextureCreator must be created once and used to create all textures.

**Solution Options**:

#### Option A: Store TextureCreator in GameContext (RECOMMENDED)
```rust
pub struct GameContext<'window> {
    pub events: Events,
    pub renderer: Canvas<Window>,
    pub texture_creator: TextureCreator<WindowContext>,  // NEW
    pub screen: Surface<'window>,
}
```

**Pros**:
- Centralized texture creation
- Easy to pass to UI initialization
- Matches SDL2 best practices

**Cons**:
- Lifetime complexity (texture_creator tied to renderer)
- GameContext becomes more complex

#### Option B: Pass TextureCreator as parameter
```rust
fn initialize_ui<'tc>(
    texture_creator: &'tc TextureCreator<WindowContext>,
    gd: &GameData
) -> UiLayer<'tc>
```

**Pros**:
- More flexible
- Explicit about dependencies

**Cons**:
- Need to thread TextureCreator through many function calls
- More verbose

**DECISION**: Use Option A - Store in GameContext

---

### 3. UI Struct Refactoring Steps

#### Step 1: Add Lifetime Parameters to All UI Structs

**Files to modify**: `src/ui.rs`

```rust
// BEFORE
pub struct MousePointer {
    frame_idx: usize,
    textures: Vec<Vec<Texture>>,
    cursor_type: MousePointerType,
    rect: Rect,
}

// AFTER
pub struct MousePointer<'tc> {
    frame_idx: usize,
    textures: Vec<Vec<Texture<'tc>>>,
    cursor_type: MousePointerType,
    rect: Rect,
}
```

Apply to:
- `MousePointer` → `MousePointer<'tc>`
- `MiniMap` → `MiniMap<'tc>`
- `SelectionPanel` → `SelectionPanel<'tc>`
- `UiLayer` → `UiLayer<'tc>`

#### Step 2: Update Constructor Signatures

```rust
// BEFORE
impl MousePointer {
    pub fn new(renderer: &mut Renderer, gd: &GameData) -> Self

// AFTER
impl<'tc> MousePointer<'tc> {
    pub fn new(texture_creator: &'tc TextureCreator<WindowContext>, gd: &GameData) -> Self
```

#### Step 3: Update Method Signatures

Render methods already updated to use `Canvas<Window>`:
```rust
impl<'tc> MousePointer<'tc> {
    pub fn render(&self, renderer: &mut Canvas<Window>) {
        // Already correct - Canvas not TextureCreator
    }
}
```

---

### 4. GameContext Refactoring

**File**: `src/lib.rs`

#### Step 1: Update GameContext struct

```rust
pub struct GameContext<'window> {
    pub events: Events,
    pub renderer: Canvas<Window>,
    pub texture_creator: TextureCreator<WindowContext>,  // ADD THIS
    pub screen: Surface<'window>,
}
```

#### Step 2: Update GameContext::new()

```rust
impl<'window> GameContext<'window> {
    fn new(
        events: Events,
        mut canvas: Canvas<Window>
    ) -> GameContext<'window> {
        let texture_creator = canvas.texture_creator();  // Extract before moving
        GameContext {
            events,
            renderer: canvas,
            texture_creator,
            screen: Surface::new(640, 480, PixelFormatEnum::Index8).unwrap(),
        }
    }
}
```

**ISSUE**: This won't compile because `texture_creator` borrows from `canvas`, but we're trying to move both into the struct!

**SOLUTION**: Use unsafe or restructure ownership. Better approach:

```rust
// Store Canvas, get TextureCreator on demand
impl<'window> GameContext<'window> {
    pub fn texture_creator(&self) -> TextureCreator<WindowContext> {
        self.renderer.texture_creator()
    }
}
```

**PROBLEM**: This creates a new TextureCreator each time! That's inefficient.

**BETTER SOLUTION**:
- Create TextureCreator FIRST
- Then store it separately
- Canvas and TextureCreator are independent (both come from Window)

```rust
// In run_game_loop
let window = video_subsystem.window(...).build().unwrap();
let canvas = window.into_canvas().accelerated().build().unwrap();
let texture_creator = canvas.texture_creator();

// Now both canvas and texture_creator are independent
let mut context = GameContext::new(events, canvas, texture_creator);
```

But this still has lifetime issues...

**ACTUAL SOLUTION**: Looking at SDL2 0.34 docs, the TextureCreator's lifetime is tied to the Canvas. We need to structure this carefully:

```rust
pub struct GameContext {  // No lifetime parameter needed
    pub events: Events,
    pub renderer: Canvas<Window>,
    pub screen: Surface<'static>,  // Use owned surface or 'static
}

// Separate structure for textures
pub struct TextureCache<'tc> {
    texture_creator: &'tc TextureCreator<WindowContext>,
    // cached textures
}
```

Actually, let me check the SDL2 0.34 pattern more carefully...

Looking at the system reminder for ui.rs, I can see that `grp_to_textures` takes `&'a TextureCreator<WindowContext>` and returns `Vec<Texture<'a>>`. This means:
- TextureCreator can be borrowed
- Textures have lifetime tied to TextureCreator
- TextureCreator must outlive the textures

The typical pattern in SDL2 0.34 is:
```rust
let canvas = window.into_canvas().build()?;
let texture_creator = canvas.texture_creator();
// texture_creator's lifetime is independent of canvas
// but textures created from it have lifetime tied to texture_creator

let texture = texture_creator.create_texture(...)?;
// texture has lifetime tied to texture_creator
```

So the solution is:
1. Create Canvas
2. Create TextureCreator from Canvas
3. Create all Textures from TextureCreator
4. UI structs store Textures with lifetimes
5. TextureCreator must be stored somewhere that outlives the UI

---

### 5. Concrete Implementation Plan

#### Phase 1: Fix GameContext (File: src/lib.rs)

**Step 1.1**: Modify GameContext to separate Canvas and texture storage

```rust
// Option 1: GameContext doesn't own textures
pub struct GameContext {
    pub events: Events,
    pub renderer: Canvas<Window>,
    pub screen: Surface<'static>,
}

// Textures are owned by View/Layer implementations
```

**Step 1.2**: Create helper to get TextureCreator

```rust
impl GameContext {
    pub fn create_texture_creator(&self) -> TextureCreator<WindowContext> {
        self.renderer.texture_creator()
    }
}
```

#### Phase 2: Fix UI Structs (File: src/ui.rs)

**Step 2.1**: Add lifetime parameters to all structs

Order matters due to dependencies:

1. `MousePointer<'tc>` (no dependencies)
2. `MiniMap<'tc>` (no dependencies)
3. `SelectionPanel<'tc>` (no dependencies)
4. `UiLayer<'tc>` (depends on MousePointer, MiniMap, SelectionPanel)

**Step 2.2**: Update all `impl` blocks

```rust
impl<'tc> MousePointer<'tc> {
    pub fn new(
        texture_creator: &'tc TextureCreator<WindowContext>,
        gd: &GameData
    ) -> Self {
        // Use texture_creator to create textures
        let all_texts = ... grp_to_textures(texture_creator, ...);
        MousePointer {
            textures: all_texts,
            // ...
        }
    }
}
```

**Step 2.3**: Update UiLayer::new to accept and pass TextureCreator

```rust
impl<'tc> UiLayer<'tc> {
    pub fn new(
        texture_creator: &'tc TextureCreator<WindowContext>,
        ctx: &mut GameContext,
        gd: &GameData
    ) -> Self {
        let mp = MousePointer::new(texture_creator, gd);
        let minimap = MiniMap::new(texture_creator, ctx, gd);
        let selection_panel = SelectionPanel::new(texture_creator, gd);
        let hud_texture = palimg_to_texture(
            texture_creator,
            640, 480,
            &hud.data,
            &hud.palette
        );

        UiLayer {
            mp,
            hud_texture,
            minimap,
            selection_panel,
            // ...
        }
    }
}
```

#### Phase 3: Fix LayerTrait (File: src/lib.rs)

The trait itself doesn't need lifetime parameters since it's object-safe:

```rust
pub trait LayerTrait {
    fn render(&self, renderer: &mut Canvas<Window>);  // Already correct
    fn update(&mut self, gd: &GameData, gc: &mut GameContext, state: &mut GameState);
    fn generate_events(&mut self, gd: &GameData, gc: &GameContext, state: &GameState) -> Vec<GameEvents>;
    fn process_event(&mut self, event: &GameEvents) -> bool;
}
```

But implementations need lifetimes:

```rust
impl<'tc> LayerTrait for UiLayer<'tc> {
    fn render(&self, renderer: &mut Canvas<Window>) {
        // ...
    }
    // ...
}
```

#### Phase 4: Fix View Initialization

Where UiLayer is created, we need to pass the TextureCreator:

```rust
// In bin files or initialization code
pub fn init(gd: &GameData, ctx: &mut GameContext, _state: &mut GameState) -> Box<dyn View> {
    let texture_creator = ctx.renderer.texture_creator();
    let ui_layer = UiLayer::new(&texture_creator, ctx, gd);

    // ... create view with ui_layer
}
```

**PROBLEM**: The texture_creator is created locally and will be dropped! Textures will dangle!

**SOLUTION**: We MUST store the TextureCreator somewhere that outlives the View.

**FINAL SOLUTION**: Store TextureCreator in the View itself or in a wrapper:

```rust
pub struct GameView<'tc> {
    texture_creator: TextureCreator<WindowContext>,
    ui_layer: UiLayer<'tc>,
    // ...
}

impl<'tc> GameView<'tc> {
    pub fn new(canvas: &Canvas<Window>, gd: &GameData) -> Self {
        let texture_creator = canvas.texture_creator();
        let ui_layer = UiLayer::new(&texture_creator, ...);
        GameView {
            texture_creator,
            ui_layer,
        }
    }
}
```

---

### 6. Secondary Issues to Fix

#### Issue 6.1: Rand API Change (Files: src/iscriptsys.rs)

**Problem**: `gen_range(min, max)` is now `gen_range(min..max)` or `gen_range(min..=max)`

```rust
// BEFORE
let r = ::rand::thread_rng().gen_range(minticks, maxticks+1);

// AFTER
let r = ::rand::thread_rng().gen_range(minticks..=maxticks);
```

Occurrences:
- src/iscriptsys.rs:334
- src/iscriptsys.rs:362

#### Issue 6.2: Config API Change (File: src/lib.rs)

**Problem**: config 0.14 has different API

```rust
// BEFORE
let mut c = config::Config::new();
let scdata_path = c.get_str("scdata_path").expect(...);

// AFTER
use config::{Config, File};
let c = Config::builder()
    .add_source(File::with_name("config.toml"))
    .build()
    .unwrap();
let scdata_path = c.get_string("scdata_path").expect(...);
```

#### Issue 6.3: SDL2 Rect API Change (File: src/ui.rs:170)

**Problem**: `Rect::contains()` method signature changed

```rust
// BEFORE
if !self.mmap_rect.contains(*screen_pt) {

// AFTER
if !self.mmap_rect.contains_point(*screen_pt) {
```

#### Issue 6.4: Module Path Issues (Various files)

Need to add `use` statements:
- `use crate::unit_ecs::UnitComponents;`
- `use sdl2::rect::Point;`
- `use crate::render;`

---

## Execution Order

### Phase 1: Quick Fixes (30 minutes)
1. ✅ Fix rand API (gen_range)
2. ✅ Fix config API
3. ✅ Fix SDL2 Rect API (contains → contains_point)
4. ✅ Fix module imports

### Phase 2: Core Architecture (2-3 hours)
5. ⏸ Design TextureCreator ownership strategy
6. ⏸ Implement TextureCreator storage in View or separate struct
7. ⏸ Update GameContext if needed

### Phase 3: UI Refactoring (3-4 hours)
8. ⏸ Add lifetimes to UI structs (MousePointer, MiniMap, SelectionPanel, UiLayer)
9. ⏸ Update all constructors to use TextureCreator
10. ⏸ Update LayerTrait implementations

### Phase 4: Integration & Testing (1-2 hours)
11. ⏸ Update bin files that create Views
12. ⏸ Test compilation
13. ⏸ Fix remaining lifetime/borrowing issues

---

## Recommended Approach

**Start with Phase 1** (quick wins) to reduce error count, then tackle the architectural issues in Phase 2-3.

**Alternative**: If you're planning to switch to Vulkan soon, consider:
1. Do Phase 1 only (fix non-SDL issues)
2. Skip complex SDL2 refactoring
3. Design graphics abstraction layer instead
4. Implement Vulkan renderer

This avoids investing significant effort in SDL2 code that will be replaced.

---

## Decision Points

Before proceeding, decide:

1. **How much SDL2 work to do?**
   - [ ] Full fix (all phases) - ~6-9 hours
   - [ ] Minimal fix (Phase 1 only) - ~30 minutes
   - [ ] Skip SDL2, start graphics abstraction - depends on design time

2. **TextureCreator storage strategy?**
   - [ ] Store in View structs
   - [ ] Store in GameContext
   - [ ] Create wrapper/manager struct

3. **Test without assets?**
   - [ ] Focus on compilation only
   - [ ] Need to test with StarCraft assets

Let me know your preference and I'll proceed accordingly!
