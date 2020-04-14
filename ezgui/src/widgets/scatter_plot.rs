use crate::{
    Color, Drawable, EventCtx, GeomBatch, GfxCtx, Line, ScreenDims, ScreenPt, ScreenRectangle,
    Text, TextExt, Widget, WidgetImpl, WidgetOutput,
};
use geom::{Angle, Circle, Distance, Duration, Pt2D};

// TODO This is tuned for the trip time comparison right now.
// - Generic types for x and y axis
// - number of labels
// - rounding behavior
// - forcing the x and y axis to be on the same scale, be drawn as a square
// - coloring the better/worse

pub struct ScatterPlot {
    draw: Drawable,

    max: Duration,
    x_name: String,
    y_name: String,

    top_left: ScreenPt,
    dims: ScreenDims,
}

impl ScatterPlot {
    pub fn new(
        ctx: &mut EventCtx,
        x_name: &str,
        y_name: &str,
        points: Vec<(Duration, Duration)>,
    ) -> Widget {
        if points.is_empty() {
            return Widget::nothing();
        }

        let actual_max = *points.iter().map(|(b, a)| a.max(b)).max().unwrap();
        // Excluding 0
        let num_labels = 5;
        let (max, labels) = make_intervals(actual_max, num_labels);

        // We want a nice square so the scales match up.
        let width = 500.0;
        let height = width;

        let mut batch = GeomBatch::new();
        batch.autocrop_dims = false;

        // Grid lines
        let thickness = Distance::meters(2.0);
        for i in 1..num_labels {
            let x = (i as f64) / (num_labels as f64) * width;
            let y = (i as f64) / (num_labels as f64) * height;
            // Horizontal
            batch.push(
                Color::grey(0.5),
                geom::Line::new(Pt2D::new(0.0, y), Pt2D::new(width, y)).make_polygons(thickness),
            );
            // Vertical
            batch.push(
                Color::grey(0.5),
                geom::Line::new(Pt2D::new(x, 0.0), Pt2D::new(x, height)).make_polygons(thickness),
            );
        }

        let circle = Circle::new(Pt2D::new(0.0, 0.0), Distance::meters(4.0)).to_polygon();
        for (b, a) in points {
            let pt = Pt2D::new((b / max) * width, (1.0 - (a / max)) * height);
            // TODO Could color circles by mode
            let color = if a == b {
                Color::YELLOW.alpha(0.5)
            } else if a < b {
                Color::GREEN.alpha(0.9)
            } else {
                Color::RED.alpha(0.9)
            };
            batch.push(color, circle.translate(pt.x(), pt.y()));
        }
        let plot = Widget::new(Box::new(ScatterPlot {
            dims: batch.get_dims(),
            draw: ctx.upload(batch),
            max,
            x_name: x_name.to_string(),
            y_name: y_name.to_string(),
            top_left: ScreenPt::new(0.0, 0.0),
        }));

        let y_axis = Widget::col(
            labels
                .iter()
                .rev()
                .map(|x| Line(x.to_string()).small().draw(ctx))
                .collect(),
        )
        .evenly_spaced();
        let y_label = {
            let mut label = GeomBatch::new();
            for (color, poly) in Text::from(Line(format!("{} (minutes)", y_name)))
                .render_ctx(ctx)
                .consume()
            {
                label.fancy_push(color, poly.rotate(Angle::new_degs(90.0)));
            }
            Widget::draw_batch(ctx, label.autocrop()).centered_vert()
        };

        let x_axis = Widget::row(
            labels
                .iter()
                .map(|x| Line(x.to_string()).small().draw(ctx))
                .collect(),
        )
        .evenly_spaced();
        let x_label = format!("{} (minutes)", x_name)
            .draw_text(ctx)
            .centered_horiz();

        // It's a bit of work to make both the x and y axis line up with the plot. :)
        let plot_width = plot.get_width_for_forcing();
        Widget::row(vec![Widget::col(vec![
            Widget::row(vec![y_label, y_axis, plot]),
            Widget::col(vec![x_axis, x_label])
                .force_width(plot_width)
                .align_right(),
        ])])
    }
}

impl WidgetImpl for ScatterPlot {
    fn get_dims(&self) -> ScreenDims {
        self.dims
    }

    fn set_pos(&mut self, top_left: ScreenPt) {
        self.top_left = top_left;
    }

    fn event(&mut self, _ctx: &mut EventCtx, _output: &mut WidgetOutput) {}

    fn draw(&self, g: &mut GfxCtx) {
        g.redraw_at(self.top_left, &self.draw);

        if let Some(cursor) = g.canvas.get_cursor_in_screen_space() {
            let rect = ScreenRectangle::top_left(self.top_left, self.dims);
            if let Some((pct_x, pct_y)) = rect.pt_to_percent(cursor) {
                let thickness = Distance::meters(2.0);
                let mut batch = GeomBatch::new();
                // Horizontal
                batch.push(
                    Color::WHITE,
                    geom::Line::new(Pt2D::new(rect.x1, cursor.y), Pt2D::new(cursor.x, cursor.y))
                        .make_polygons(thickness),
                );
                // Vertical
                batch.push(
                    Color::WHITE,
                    geom::Line::new(Pt2D::new(cursor.x, rect.y2), Pt2D::new(cursor.x, cursor.y))
                        .make_polygons(thickness),
                );

                g.fork_screenspace();
                let draw = g.upload(batch);
                g.redraw(&draw);
                g.draw_mouse_tooltip(Text::from_multiline(vec![
                    Line(format!("{}: {}", self.x_name, pct_x * self.max)),
                    Line(format!("{}: {}", self.y_name, (1.0 - pct_y) * self.max)),
                ]));
                g.unfork();
            }
        }
    }
}

// TODO Do something fancier? http://vis.stanford.edu/papers/tick-labels
fn make_intervals(actual_max: Duration, num_labels: usize) -> (Duration, Vec<usize>) {
    // Example: 43 minutes, max 5 labels... raw_mins_per_interval is 8.6
    let raw_mins_per_interval =
        (actual_max.num_minutes_rounded_down() as f64) / (num_labels as f64);
    // So then this rounded up to 10 minutes
    let mins_per_interval = Duration::seconds(60.0 * raw_mins_per_interval)
        .round_up(Duration::minutes(5))
        .num_minutes_rounded_down();

    (
        actual_max.round_up(Duration::minutes(mins_per_interval)),
        (0..=num_labels).map(|i| i * mins_per_interval).collect(),
    )
}