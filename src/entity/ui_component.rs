pub enum UIComponent {
    Text {
        string: Box<dyn Fn() -> String>,
        pixel_size: f32,
        color: (f32, f32, f32),
        line_height: f32,
    },
    Rect {
        size: (f32, f32),
        background: (f32, f32, f32),
    },
}
