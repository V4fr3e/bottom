#[derive(Debug, Clone)]
pub struct Chart<'a> {
    /// A block to display around the widget eventually
    block: Option<Block<'a>>,
    /// The horizontal axis
    x_axis: Axis<'a>,
    /// The vertical axis
    y_axis: Axis<'a>,
    /// A reference to the datasets
    datasets: Vec<Dataset<'a>>,
    /// The widget base style
    style: Style,
    /// Constraints used to determine whether the legend should be shown or not
    hidden_legend_constraints: (Constraint, Constraint),
}

impl<'a> Chart<'a> {
    pub fn new(datasets: Vec<Dataset<'a>>) -> Chart<'a> {
        Chart {
            block: None,
            x_axis: Axis::default(),
            y_axis: Axis::default(),
            style: Default::default(),
            datasets,
            hidden_legend_constraints: (Constraint::Ratio(1, 4), Constraint::Ratio(1, 4)),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Chart<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Chart<'a> {
        self.style = style;
        self
    }

    pub fn x_axis(mut self, axis: Axis<'a>) -> Chart<'a> {
        self.x_axis = axis;
        self
    }

    pub fn y_axis(mut self, axis: Axis<'a>) -> Chart<'a> {
        self.y_axis = axis;
        self
    }

    /// Set the constraints used to determine whether the legend should be shown or not.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tui::widgets::Chart;
    /// # use tui::layout::Constraint;
    /// let constraints = (
    ///     Constraint::Ratio(1, 3),
    ///     Constraint::Ratio(1, 4)
    /// );
    /// // Hide the legend when either its width is greater than 33% of the total widget width
    /// // or if its height is greater than 25% of the total widget height.
    /// let _chart: Chart = Chart::new(vec![])
    ///     .hidden_legend_constraints(constraints);
    /// ```
    pub fn hidden_legend_constraints(mut self, constraints: (Constraint, Constraint)) -> Chart<'a> {
        self.hidden_legend_constraints = constraints;
        self
    }

    /// Compute the internal layout of the chart given the area. If the area is too small some
    /// elements may be automatically hidden
    fn layout(&self, area: Rect) -> ChartLayout {
        let mut layout = ChartLayout::default();
        if area.height == 0 || area.width == 0 {
            return layout;
        }
        let mut x = area.left();
        let mut y = area.bottom() - 1;

        if self.x_axis.labels.is_some() && y > area.top() {
            layout.label_x = Some(y);
            y -= 1;
        }

        layout.label_y = self.y_axis.labels.as_ref().and(Some(x));
        x += self.max_width_of_labels_left_of_y_axis(area);

        if self.x_axis.labels.is_some() && y > area.top() {
            layout.axis_x = Some(y);
            y -= 1;
        }

        if self.y_axis.labels.is_some() && x + 1 < area.right() {
            layout.axis_y = Some(x);
            x += 1;
        }

        if x < area.right() && y > 1 {
            layout.graph_area = Rect::new(x, area.top(), area.right() - x, y - area.top() + 1);
        }

        if let Some(ref title) = self.x_axis.title {
            let w = title.width() as u16;
            if w < layout.graph_area.width && layout.graph_area.height > 2 {
                layout.title_x = Some((x + layout.graph_area.width - w, y));
            }
        }

        if let Some(ref title) = self.y_axis.title {
            let w = title.width() as u16;
            if w + 1 < layout.graph_area.width && layout.graph_area.height > 2 {
                layout.title_y = Some((x, area.top()));
            }
        }

        if let Some(inner_width) = self.datasets.iter().map(|d| d.name.width() as u16).max() {
            let legend_width = inner_width + 2;
            let legend_height = self.datasets.len() as u16 + 2;
            let max_legend_width = self
                .hidden_legend_constraints
                .0
                .apply(layout.graph_area.width);
            let max_legend_height = self
                .hidden_legend_constraints
                .1
                .apply(layout.graph_area.height);
            if inner_width > 0
                && legend_width < max_legend_width
                && legend_height < max_legend_height
            {
                layout.legend_area = Some(Rect::new(
                    layout.graph_area.right() - legend_width,
                    layout.graph_area.top(),
                    legend_width,
                    legend_height,
                ));
            }
        }
        layout
    }

    fn max_width_of_labels_left_of_y_axis(&self, area: Rect) -> u16 {
        let mut max_width = self
            .y_axis
            .labels
            .as_ref()
            .map(|l| l.iter().map(Span::width).max().unwrap_or_default() as u16)
            .unwrap_or_default();
        if let Some(ref x_labels) = self.x_axis.labels {
            if !x_labels.is_empty() {
                max_width = max(max_width, x_labels[0].content.width() as u16);
            }
        }
        // labels of y axis and first label of x axis can take at most 1/3rd of the total width
        max_width.min(area.width / 3)
    }

    fn render_x_labels(
        &mut self, buf: &mut Buffer, layout: &ChartLayout, chart_area: Rect, graph_area: Rect,
    ) {
        let y = match layout.label_x {
            Some(y) => y,
            None => return,
        };
        let labels = self.x_axis.labels.as_ref().unwrap();
        let labels_len = labels.len() as u16;
        if labels_len < 2 {
            return;
        }
        let width_between_ticks = graph_area.width / (labels_len - 1);
        for (i, label) in labels.iter().enumerate() {
            let label_width = label.width() as u16;
            let label_width = if i == 0 {
                // the first label is put between the left border of the chart and the y axis.
                graph_area
                    .left()
                    .saturating_sub(chart_area.left())
                    .min(label_width)
            } else {
                // other labels are put on the left of each tick on the x axis
                width_between_ticks.min(label_width)
            };
            buf.set_span(
                graph_area.left() + i as u16 * width_between_ticks - label_width,
                y,
                label,
                label_width,
            );
        }
    }

    fn render_y_labels(
        &mut self, buf: &mut Buffer, layout: &ChartLayout, chart_area: Rect, graph_area: Rect,
    ) {
        let x = match layout.label_y {
            Some(x) => x,
            None => return,
        };
        let labels = self.y_axis.labels.as_ref().unwrap();
        let labels_len = labels.len() as u16;
        let label_width = graph_area.left().saturating_sub(chart_area.left());
        for (i, label) in labels.iter().enumerate() {
            let dy = i as u16 * (graph_area.height - 1) / (labels_len - 1);
            if dy < graph_area.bottom() {
                buf.set_span(x, graph_area.bottom() - 1 - dy, label, label_width as u16);
            }
        }
    }
}

impl<'a> Widget for Chart<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        if area.area() == 0 {
            return;
        }
        buf.set_style(area, self.style);
        // Sample the style of the entire widget. This sample will be used to reset the style of
        // the cells that are part of the components put on top of the graph area (i.e legend and
        // axis names).
        let original_style = buf.get(area.left(), area.top()).style();

        let chart_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        let layout = self.layout(chart_area);
        let graph_area = layout.graph_area;
        if graph_area.width < 1 || graph_area.height < 1 {
            return;
        }

        self.render_x_labels(buf, &layout, chart_area, graph_area);
        self.render_y_labels(buf, &layout, chart_area, graph_area);

        if let Some(y) = layout.axis_x {
            for x in graph_area.left()..graph_area.right() {
                buf.get_mut(x, y)
                    .set_symbol(symbols::line::HORIZONTAL)
                    .set_style(self.x_axis.style);
            }
        }

        if let Some(x) = layout.axis_y {
            for y in graph_area.top()..graph_area.bottom() {
                buf.get_mut(x, y)
                    .set_symbol(symbols::line::VERTICAL)
                    .set_style(self.y_axis.style);
            }
        }

        if let Some(y) = layout.axis_x {
            if let Some(x) = layout.axis_y {
                buf.get_mut(x, y)
                    .set_symbol(symbols::line::BOTTOM_LEFT)
                    .set_style(self.x_axis.style);
            }
        }

        for dataset in &self.datasets {
            Canvas::default()
                .background_color(self.style.bg.unwrap_or(Color::Reset))
                .x_bounds(self.x_axis.bounds)
                .y_bounds(self.y_axis.bounds)
                .marker(dataset.marker)
                .paint(|ctx| {
                    ctx.draw(&Points {
                        coords: dataset.data,
                        color: dataset.style.fg.unwrap_or(Color::Reset),
                    });
                    if let GraphType::Line = dataset.graph_type {
                        for data in dataset.data.windows(2) {
                            ctx.draw(&Line {
                                x1: data[0].0,
                                y1: data[0].1,
                                x2: data[1].0,
                                y2: data[1].1,
                                color: dataset.style.fg.unwrap_or(Color::Reset),
                            })
                        }
                    }
                })
                .render(graph_area, buf);
        }

        if let Some(legend_area) = layout.legend_area {
            buf.set_style(legend_area, original_style);
            Block::default()
                .borders(Borders::ALL)
                .render(legend_area, buf);
            for (i, dataset) in self.datasets.iter().enumerate() {
                buf.set_string(
                    legend_area.x + 1,
                    legend_area.y + 1 + i as u16,
                    &dataset.name,
                    dataset.style,
                );
            }
        }

        if let Some((x, y)) = layout.title_x {
            let title = self.x_axis.title.unwrap();
            let width = graph_area.right().saturating_sub(x);
            buf.set_style(
                Rect {
                    x,
                    y,
                    width,
                    height: 1,
                },
                original_style,
            );
            buf.set_spans(x, y, &title, width);
        }

        if let Some((x, y)) = layout.title_y {
            let title = self.y_axis.title.unwrap();
            let width = graph_area.right().saturating_sub(x);
            buf.set_style(
                Rect {
                    x,
                    y,
                    width,
                    height: 1,
                },
                original_style,
            );
            buf.set_spans(x, y, &title, width);
        }
    }
}