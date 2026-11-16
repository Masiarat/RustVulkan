# RustVulkan

An experimental Vulkan renderer written in Rust. The project focuses on low-level GPU programming, explicit resource management, synchronization, and integrating an immediate-mode UI with a custom renderer.

## Features

- Vulkan context, physical-device selection, queues, swapchain, and frame synchronization
- Compute pipeline generating a configurable color gradient
- Indexed geometry rendering with GPU vertex and index buffers
- Vulkan image layout transitions, staging uploads, descriptor allocation, and deferred cleanup
- `egui` integration through a custom Vulkan painter
- UI panel for stress-testing GPU texture creation and cleanup
- Slang shaders compiled to SPIR-V during the Cargo build

## Technology

- Rust 2024
- Vulkan via [`ash`](https://crates.io/crates/ash)
- [`winit`](https://crates.io/crates/winit) for window and event handling
- [`egui`](https://github.com/emilk/egui) for the debug UI
- [`gpu-allocator`](https://crates.io/crates/gpu-allocator) for Vulkan memory allocation
- Slang shaders compiled with `slangc`

## Architecture

```text
src/
├── app.rs                  # Window lifecycle, events, and egui
├── renderer.rs             # Frame rendering orchestration
├── renderer/
│   ├── core/               # Vulkan context, device, and queues
│   ├── pass/               # Compute, mesh, and egui render passes
│   ├── resource/           # Images, buffers, descriptors, and samplers
│   └── sync/               # Frames, swapchain, and synchronization
└── shaders/                # Slang shader sources
```

Each frame generates the gradient with a compute shader, copies it to the swapchain image, renders geometry, and then draws the egui interface.

## Requirements

- Rust stable with Rust 2024 edition support
- A Vulkan-capable GPU and driver
- `slangc` available in `PATH`

## Running

```bash
git clone https://github.com/Masiarat/RustVulkan.git
cd RustVulkan
cargo run
```

`build.rs` automatically compiles every `.slang` file in `src/shaders/` to SPIR-V. The application opens an 800 × 600 window where the gradient colors and GPU texture allocation can be tested.

## Project status

This is an educational portfolio project focused on the renderer foundation and GPU resource lifetime. Possible next steps include swapchain recreation on window resize, camera support, model loading, validation-layer tooling, and a material system.
