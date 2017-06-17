# Vulkano Text

Render text with the DejaVu font using the Vulkano library.

##[Documentation](https://docs.rs/vulkano_text)

## Usage:

Add to your Cargo.toml: 
```
vulkano_text = "0.2"
```

Below are relevant lines taken from [window.rs](examples/window.rs)

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
draw_text.queue_text(20.0, 200.0, 190.0, [1.0, 0.0, 0.0, 1.0], "Hello world!");
draw_text.queue_text(x, 350.0, 70.0, [0.51, 0.6, 0.74, 1.0], "Lenny: ( ͡° ͜ʖ ͡°)");
```

Call update_text_cache on the AutoCommandBufferBuilder before render pass
```
.update_text_cache(&mut draw_text)
```

Call draw_text on the AutoCommandBufferBuilder during render pass.
```
.draw_text(&mut draw_text, queue.clone(), width, height)
```

Result:
![Result:](screenshot.png)
