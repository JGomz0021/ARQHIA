//! Logo de ARQHIA dibujado con `canvas` (v0.9.6).
//!
//! La geometría replica `assets/icon.svg` (rounded-square + "A" en tres
//! cápsulas) para que el logotipo de Home case con el icono del sistema.
//! Sin dependencias de decodificación de imagen (no se añade `image`).

use iced::widget::canvas::{self, Geometry, LineCap, Path, Stroke};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme, mouse};

const BG_TOP: Color = Color::from_rgb(0.106, 0.129, 0.251);
const BG_BOT: Color = Color::from_rgb(0.043, 0.055, 0.110);
const A_TOP: Color = Color::from_rgb(0.404, 0.910, 0.976);
const A_BOT: Color = Color::from_rgb(0.655, 0.545, 0.980);

/// Elemento del logo a `size` px lógicos.
pub fn view<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    canvas::Canvas::new(Logo { size })
        .width(size)
        .height(size)
        .into()
}

struct Logo {
    size: f32,
}

fn linear(top: Color, bot: Color, size: f32) -> canvas::Gradient {
    canvas::gradient::Linear::new(Point::new(0.0, 0.0), Point::new(0.0, size))
        .add_stop(0.0, top)
        .add_stop(1.0, bot)
        .into()
}

impl<Message> canvas::Program<Message> for Logo {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = canvas::Frame::new(renderer, Size::new(self.size, self.size));
        let k = self.size / 256.0;

        let bg = Path::rounded_rectangle(
            Point::new(0.0, 0.0),
            Size::new(self.size, self.size),
            (56.0 * k).into(),
        );
        frame.fill(&bg, linear(BG_TOP, BG_BOT, self.size));

        let accent = linear(A_TOP, A_BOT, self.size);
        let legs = Stroke {
            style: accent.into(),
            width: 34.0 * k,
            line_cap: LineCap::Round,
            ..Stroke::default()
        };
        let bar = Stroke {
            style: accent.into(),
            width: 26.0 * k,
            line_cap: LineCap::Round,
            ..Stroke::default()
        };
        let p = |x: f32, y: f32| Point::new(x * k, y * k);
        frame.stroke(&Path::line(p(128.0, 50.0), p(66.0, 206.0)), legs);
        frame.stroke(&Path::line(p(128.0, 50.0), p(190.0, 206.0)), legs);
        frame.stroke(&Path::line(p(86.0, 156.0), p(170.0, 156.0)), bar);

        vec![frame.into_geometry()]
    }
}
