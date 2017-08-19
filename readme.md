# Vulkano Text [![](https://img.shields.io/crates/v/vulkano_text.svg)](https://crates.io/crates/vulkano-text)

Render text with the DejaVu font using the Vulkano library.

## [Documentation](https://docs.rs/vulkano_text)

## Usage:

Below are relevant lines taken from the [triangle.rs](examples/triangle.rs) example.

Import the library:
```
extern crate vulkano_text;
use vulkano_text::{DrawText, DrawTextTrait, UpdateTextCache};
```

Create DrawText:
```
let mut draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);
```

Specify text to draw by calling queue_text:
```
draw_text.queue_text(200.0, 50.0, 20.0, [1.0, 1.0, 1.0, 1.0], "The quick brown fox jumps over the lazy dog.");
draw_text.queue_text(20.0, 200.0, 190.0, [0.0, 1.0, 1.0, 1.0], "Hello world!");
draw_text.queue_text(x, 350.0, 70.0, [0.51, 0.6, 0.74, 1.0], "Lenny: ( ͡° ͜ʖ ͡°)");
draw_text.queue_text(50.0, 350.0, 70.0, [1.0, 1.0, 1.0, 1.0], "Overlap");
```

Call update_text_cache on the AutoCommandBufferBuilder before render pass
```
.update_text_cache(&mut draw_text)
```

Call draw_text on the AutoCommandBufferBuilder during render pass.
```
.draw_text(&mut draw_text, width, height)
```

Result:
![Result:](screenshot.png)
