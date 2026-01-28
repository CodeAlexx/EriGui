use crate::{Button, Icon, TextInput};
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use erigui_core::{
    DrawContext, Event, EventResult, Key, KeyPressEvent, LayoutConstraints, MouseButton,
    MouseButtonEvent, Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DateTimePickerMode {
    Date,
    Time,
    DateTime,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DateFormat {
    YMD, // YYYY-MM-DD
    DMY, // DD/MM/YYYY
    MDY, // MM/DD/YYYY
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeFormat {
    H24, // 24-hour format
    H12, // 12-hour format with AM/PM
}

pub struct DateTimePicker {
    state: WidgetState,
    mode: DateTimePickerMode,
    date_format: DateFormat,
    time_format: TimeFormat,

    // Current value
    selected_datetime: NaiveDateTime,

    // UI state
    show_calendar: bool,
    show_time_picker: bool,
    calendar_rect: Rect,
    time_picker_rect: Rect,

    // Calendar state
    viewing_year: i32,
    viewing_month: u32,
    hovered_day: Option<u32>,

    // Time picker state
    editing_hour: bool,
    editing_minute: bool,
    hour_input: String,
    minute_input: String,
    am_pm_selected: bool, // false = AM, true = PM

    // Components
    input: TextInput,
    calendar_button: Button,

    // Callbacks
    on_change: Option<Box<dyn FnMut(NaiveDateTime)>>,
}

impl DateTimePicker {
    pub fn new(id: WidgetId) -> Self {
        let now = Local::now().naive_local();
        let mut picker = Self {
            state: WidgetState::new(id),
            mode: DateTimePickerMode::Date,
            date_format: DateFormat::YMD,
            time_format: TimeFormat::H24,
            selected_datetime: now,
            show_calendar: false,
            show_time_picker: false,
            calendar_rect: Rect::default(),
            time_picker_rect: Rect::default(),
            viewing_year: now.year(),
            viewing_month: now.month(),
            hovered_day: None,
            editing_hour: false,
            editing_minute: false,
            hour_input: format!("{:02}", now.hour()),
            minute_input: format!("{:02}", now.minute()),
            am_pm_selected: now.hour() >= 12,
            input: TextInput::new(WidgetId::default()).with_placeholder("Select date..."),
            calendar_button: Button::new(WidgetId::default(), "📅"),
            on_change: None,
        };
        picker.update_input_text();
        picker
    }

    pub fn with_mode(mut self, mode: DateTimePickerMode) -> Self {
        self.mode = mode;
        self.update_placeholder();
        self
    }

    pub fn with_date_format(mut self, format: DateFormat) -> Self {
        self.date_format = format;
        self.update_input_text();
        self
    }

    pub fn with_time_format(mut self, format: TimeFormat) -> Self {
        self.time_format = format;
        self.update_input_text();
        self.update_time_inputs();
        self
    }

    pub fn with_value(mut self, datetime: NaiveDateTime) -> Self {
        self.selected_datetime = datetime;
        self.viewing_year = datetime.year();
        self.viewing_month = datetime.month();
        self.update_input_text();
        self.update_time_inputs();
        self
    }

    pub fn with_on_change<F: FnMut(NaiveDateTime) + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    pub fn get_value(&self) -> NaiveDateTime {
        self.selected_datetime
    }

    pub fn set_value(&mut self, datetime: NaiveDateTime) {
        self.selected_datetime = datetime;
        self.viewing_year = datetime.year();
        self.viewing_month = datetime.month();
        self.update_input_text();
        self.update_time_inputs();
        self.notify_change();
    }

    fn update_placeholder(&mut self) {
        let placeholder = match self.mode {
            DateTimePickerMode::Date => match self.date_format {
                DateFormat::YMD => "YYYY-MM-DD",
                DateFormat::DMY => "DD/MM/YYYY",
                DateFormat::MDY => "MM/DD/YYYY",
            },
            DateTimePickerMode::Time => match self.time_format {
                TimeFormat::H24 => "HH:MM",
                TimeFormat::H12 => "HH:MM AM/PM",
            },
            DateTimePickerMode::DateTime => match self.date_format {
                DateFormat::YMD => "YYYY-MM-DD HH:MM",
                DateFormat::DMY => "DD/MM/YYYY HH:MM",
                DateFormat::MDY => "MM/DD/YYYY HH:MM",
            },
        };
        self.input.set_placeholder(placeholder);
    }

    fn update_input_text(&mut self) {
        let text = match self.mode {
            DateTimePickerMode::Date => self.format_date(),
            DateTimePickerMode::Time => self.format_time(),
            DateTimePickerMode::DateTime => {
                format!("{} {}", self.format_date(), self.format_time())
            }
        };
        self.input.set_text(text);
    }

    fn format_date(&self) -> String {
        let date = self.selected_datetime.date();
        match self.date_format {
            DateFormat::YMD => format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()),
            DateFormat::DMY => format!("{:02}/{:02}/{:04}", date.day(), date.month(), date.year()),
            DateFormat::MDY => format!("{:02}/{:02}/{:04}", date.month(), date.day(), date.year()),
        }
    }

    fn format_time(&self) -> String {
        let time = self.selected_datetime.time();
        match self.time_format {
            TimeFormat::H24 => format!("{:02}:{:02}", time.hour(), time.minute()),
            TimeFormat::H12 => {
                let hour = if time.hour() == 0 {
                    12
                } else if time.hour() > 12 {
                    time.hour() - 12
                } else {
                    time.hour()
                };
                let am_pm = if time.hour() < 12 { "AM" } else { "PM" };
                format!("{:02}:{:02} {}", hour, time.minute(), am_pm)
            }
        }
    }

    fn update_time_inputs(&mut self) {
        let time = self.selected_datetime.time();

        match self.time_format {
            TimeFormat::H24 => {
                self.hour_input = format!("{:02}", time.hour());
            }
            TimeFormat::H12 => {
                let hour = if time.hour() == 0 {
                    12
                } else if time.hour() > 12 {
                    time.hour() - 12
                } else {
                    time.hour()
                };
                self.hour_input = format!("{:02}", hour);
                self.am_pm_selected = time.hour() >= 12;
            }
        }

        self.minute_input = format!("{:02}", time.minute());
    }

    fn notify_change(&mut self) {
        if let Some(callback) = &mut self.on_change {
            callback(self.selected_datetime);
        }
    }

    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }

    fn first_day_of_week(year: i32, month: u32) -> u32 {
        // Get day of week for first day of month (0 = Sunday, 6 = Saturday)
        let date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        date.weekday().num_days_from_sunday()
    }

    fn draw_calendar(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.show_calendar {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.calendar_rect);

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.calendar_rect);

        // Draw shadow
        context.set_color(theme.colors.shadow);
        context.fill_rect(Rect::new(
            self.calendar_rect.x() + 2,
            self.calendar_rect.y() + 2,
            self.calendar_rect.width(),
            self.calendar_rect.height(),
        ));

        let padding = 10;
        let header_height = 30;
        let cell_size = 30;

        // Draw month/year header
        let header_rect = Rect::new(
            self.calendar_rect.x(),
            self.calendar_rect.y(),
            self.calendar_rect.width(),
            header_height,
        );

        context.set_color(theme.colors.primary);
        context.fill_rect(header_rect);

        // Draw month/year text
        context.set_color(theme.colors.background);
        let month_names = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let header_text = format!(
            "{} {}",
            month_names[(self.viewing_month - 1) as usize],
            self.viewing_year
        );
        context.draw_text(
            &header_text,
            Point::new(
                header_rect.center().x - header_text.len() as i32 * 4,
                header_rect.center().y + 4,
            ),
            theme.typography.font_size_base,
        );

        // Draw navigation arrows
        let arrow_size = 16;
        let prev_rect = Rect::new(
            header_rect.x() + padding,
            header_rect.center().y - arrow_size / 2,
            arrow_size,
            arrow_size,
        );
        let next_rect = Rect::new(
            header_rect.right() - padding - arrow_size,
            header_rect.center().y - arrow_size / 2,
            arrow_size,
            arrow_size,
        );

        Icon::draw_back_arrow(context, prev_rect, theme.colors.background);
        Icon::draw_forward_arrow(context, next_rect, theme.colors.background);

        // Draw day headers
        let day_names = ["S", "M", "T", "W", "T", "F", "S"];
        let calendar_y = header_rect.bottom() + padding;

        context.set_color(theme.colors.text_secondary);
        for (i, day) in day_names.iter().enumerate() {
            let x = self.calendar_rect.x() + padding + i as i32 * cell_size + cell_size / 2 - 4;
            let y = calendar_y + 8;
            context.draw_text(day, Point::new(x, y), theme.typography.font_size_small);
        }

        // Draw days
        let days_in_month = Self::days_in_month(self.viewing_year, self.viewing_month);
        let first_day = Self::first_day_of_week(self.viewing_year, self.viewing_month);
        let selected_date = self.selected_datetime.date();

        let days_y = calendar_y + 20;

        for day in 1..=days_in_month {
            let grid_pos = first_day + day - 1;
            let row = grid_pos / 7;
            let col = grid_pos % 7;

            let x = self.calendar_rect.x() + padding + col as i32 * cell_size;
            let y = days_y + row as i32 * cell_size;
            let day_rect = Rect::new(x, y, cell_size, cell_size);

            // Check if this is the selected day
            let is_selected = selected_date.year() == self.viewing_year
                && selected_date.month() == self.viewing_month
                && selected_date.day() == day;

            let is_hovered = self.hovered_day == Some(day);

            // Draw background
            if is_selected {
                context.set_color(theme.colors.primary);
                context.fill_ellipse(day_rect.center(), cell_size / 2 - 2, cell_size / 2 - 2, 16);
            } else if is_hovered {
                context.set_color(theme.colors.primary_hover);
                context.fill_ellipse(day_rect.center(), cell_size / 2 - 2, cell_size / 2 - 2, 16);
            }

            // Draw day number
            context.set_color(if is_selected {
                theme.colors.background
            } else {
                theme.colors.text
            });

            let day_text = day.to_string();
            let text_x = day_rect.center().x - day_text.len() as i32 * 3;
            let text_y = day_rect.center().y + 4;
            context.draw_text(
                &day_text,
                Point::new(text_x, text_y),
                theme.typography.font_size_base,
            );
        }
    }

    fn draw_time_picker(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.show_time_picker {
            return;
        }

        // Draw background
        context.set_color(theme.colors.surface);
        context.fill_rect(self.time_picker_rect);

        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rect(self.time_picker_rect);

        let padding = 20;
        let input_width = 60;
        let input_height = 30;

        let center_x = self.time_picker_rect.center().x;
        let center_y = self.time_picker_rect.center().y;

        // Draw hour input
        let hour_rect = Rect::new(
            center_x - input_width - padding / 2,
            center_y - input_height / 2,
            input_width,
            input_height,
        );

        context.set_color(if self.editing_hour {
            theme.colors.primary
        } else {
            theme.colors.border
        });
        context.draw_rect(hour_rect);

        context.set_color(theme.colors.surface_variant);
        context.fill_rect(hour_rect.inset(1));

        context.set_color(theme.colors.text);
        context.draw_text(
            &self.hour_input,
            Point::new(hour_rect.center().x - 8, hour_rect.center().y + 4),
            theme.typography.font_size_large,
        );

        // Draw colon
        context.draw_text(
            ":",
            Point::new(center_x - 4, center_y + 4),
            theme.typography.font_size_large,
        );

        // Draw minute input
        let minute_rect = Rect::new(
            center_x + padding / 2,
            center_y - input_height / 2,
            input_width,
            input_height,
        );

        context.set_color(if self.editing_minute {
            theme.colors.primary
        } else {
            theme.colors.border
        });
        context.draw_rect(minute_rect);

        context.set_color(theme.colors.surface_variant);
        context.fill_rect(minute_rect.inset(1));

        context.set_color(theme.colors.text);
        context.draw_text(
            &self.minute_input,
            Point::new(minute_rect.center().x - 8, minute_rect.center().y + 4),
            theme.typography.font_size_large,
        );

        // Draw AM/PM selector for 12-hour format
        if self.time_format == TimeFormat::H12 {
            let am_pm_y = center_y + input_height + padding;

            let am_rect = Rect::new(center_x - 40, am_pm_y, 35, 25);
            let pm_rect = Rect::new(center_x + 5, am_pm_y, 35, 25);

            // AM button
            context.set_color(if !self.am_pm_selected {
                theme.colors.primary
            } else {
                theme.colors.surface_variant
            });
            context.fill_rect(am_rect);

            context.set_color(if !self.am_pm_selected {
                theme.colors.background
            } else {
                theme.colors.text
            });
            context.draw_text(
                "AM",
                Point::new(am_rect.center().x - 8, am_rect.center().y + 4),
                theme.typography.font_size_base,
            );

            // PM button
            context.set_color(if self.am_pm_selected {
                theme.colors.primary
            } else {
                theme.colors.surface_variant
            });
            context.fill_rect(pm_rect);

            context.set_color(if self.am_pm_selected {
                theme.colors.background
            } else {
                theme.colors.text
            });
            context.draw_text(
                "PM",
                Point::new(pm_rect.center().x - 8, pm_rect.center().y + 4),
                theme.typography.font_size_base,
            );
        }
    }

    fn handle_calendar_click(&mut self, position: Point) -> bool {
        let padding = 10;
        let header_height = 30;
        let cell_size = 30;

        // Check navigation arrows
        let header_rect = Rect::new(
            self.calendar_rect.x(),
            self.calendar_rect.y(),
            self.calendar_rect.width(),
            header_height,
        );

        let arrow_size = 16;
        let prev_rect = Rect::new(
            header_rect.x() + padding,
            header_rect.center().y - arrow_size / 2,
            arrow_size,
            arrow_size,
        );
        let next_rect = Rect::new(
            header_rect.right() - padding - arrow_size,
            header_rect.center().y - arrow_size / 2,
            arrow_size,
            arrow_size,
        );

        if prev_rect.contains(position) {
            // Previous month
            if self.viewing_month == 1 {
                self.viewing_month = 12;
                self.viewing_year -= 1;
            } else {
                self.viewing_month -= 1;
            }
            return true;
        }

        if next_rect.contains(position) {
            // Next month
            if self.viewing_month == 12 {
                self.viewing_month = 1;
                self.viewing_year += 1;
            } else {
                self.viewing_month += 1;
            }
            return true;
        }

        // Check day clicks
        let calendar_y = header_rect.bottom() + padding;
        let days_y = calendar_y + 20;
        let first_day = Self::first_day_of_week(self.viewing_year, self.viewing_month);
        let days_in_month = Self::days_in_month(self.viewing_year, self.viewing_month);

        for day in 1..=days_in_month {
            let grid_pos = first_day + day - 1;
            let row = grid_pos / 7;
            let col = grid_pos % 7;

            let x = self.calendar_rect.x() + padding + col as i32 * cell_size;
            let y = days_y + row as i32 * cell_size;
            let day_rect = Rect::new(x, y, cell_size, cell_size);

            if day_rect.contains(position) {
                // Update selected date
                let new_date =
                    NaiveDate::from_ymd_opt(self.viewing_year, self.viewing_month, day).unwrap();
                self.selected_datetime =
                    NaiveDateTime::new(new_date, self.selected_datetime.time());
                self.update_input_text();
                self.notify_change();

                // Close calendar if in date-only mode
                if self.mode == DateTimePickerMode::Date {
                    self.show_calendar = false;
                }

                return true;
            }
        }

        false
    }

    fn handle_time_picker_click(&mut self, position: Point) -> bool {
        let padding = 20;
        let input_width = 60;
        let input_height = 30;

        let center_x = self.time_picker_rect.center().x;
        let center_y = self.time_picker_rect.center().y;

        // Check hour input
        let hour_rect = Rect::new(
            center_x - input_width - padding / 2,
            center_y - input_height / 2,
            input_width,
            input_height,
        );

        if hour_rect.contains(position) {
            self.editing_hour = true;
            self.editing_minute = false;
            return true;
        }

        // Check minute input
        let minute_rect = Rect::new(
            center_x + padding / 2,
            center_y - input_height / 2,
            input_width,
            input_height,
        );

        if minute_rect.contains(position) {
            self.editing_hour = false;
            self.editing_minute = true;
            return true;
        }

        // Check AM/PM buttons for 12-hour format
        if self.time_format == TimeFormat::H12 {
            let am_pm_y = center_y + input_height + padding;

            let am_rect = Rect::new(center_x - 40, am_pm_y, 35, 25);
            let pm_rect = Rect::new(center_x + 5, am_pm_y, 35, 25);

            if am_rect.contains(position) {
                self.am_pm_selected = false;
                self.update_time_from_inputs();
                return true;
            }

            if pm_rect.contains(position) {
                self.am_pm_selected = true;
                self.update_time_from_inputs();
                return true;
            }
        }

        false
    }

    fn update_time_from_inputs(&mut self) {
        let hour: u32 = self.hour_input.parse().unwrap_or(0);
        let minute: u32 = self.minute_input.parse().unwrap_or(0);

        let final_hour = match self.time_format {
            TimeFormat::H24 => hour.min(23),
            TimeFormat::H12 => {
                let h = hour.min(12).max(1);
                if h == 12 {
                    if self.am_pm_selected {
                        12
                    } else {
                        0
                    }
                } else {
                    if self.am_pm_selected {
                        h + 12
                    } else {
                        h
                    }
                }
            }
        };

        let final_minute = minute.min(59);

        let new_time = NaiveTime::from_hms_opt(final_hour, final_minute, 0).unwrap();
        self.selected_datetime = NaiveDateTime::new(self.selected_datetime.date(), new_time);
        self.update_input_text();
        self.notify_change();
    }
}

impl Widget for DateTimePicker {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let input_width = 200;
        let button_width = 30;
        let height = theme.typography.font_size_base + 16;

        Size::new(input_width + button_width + 4, height)
    }

    fn layout(&mut self, rect: Rect, theme: &Theme) {
        self.state.bounds = rect;

        let button_width = 30;

        // Layout input
        self.input.layout(
            Rect::new(
                rect.x(),
                rect.y(),
                rect.width() - button_width - 4,
                rect.height(),
            ),
            theme,
        );

        // Layout button
        self.calendar_button.layout(
            Rect::new(
                rect.right() - button_width,
                rect.y(),
                button_width,
                rect.height(),
            ),
            theme,
        );

        // Calculate popup positions
        let calendar_width = 250;
        let calendar_height = 280;

        self.calendar_rect =
            Rect::new(rect.x(), rect.bottom() + 2, calendar_width, calendar_height);

        let time_picker_width = 200;
        let time_picker_height = if self.time_format == TimeFormat::H12 {
            120
        } else {
            80
        };

        self.time_picker_rect = if self.mode == DateTimePickerMode::DateTime {
            Rect::new(
                rect.x() + calendar_width + 10,
                rect.bottom() + 2,
                time_picker_width,
                time_picker_height,
            )
        } else {
            Rect::new(
                rect.x(),
                rect.bottom() + 2,
                time_picker_width,
                time_picker_height,
            )
        };
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        // Draw input and button
        self.input.draw(context, theme);
        self.calendar_button.draw(context, theme);

        // Draw popups
        if self.mode != DateTimePickerMode::Time {
            self.draw_calendar(context, theme);
        }

        if self.mode != DateTimePickerMode::Date {
            self.draw_time_picker(context, theme);
        }
    }

    fn handle_event(&mut self, event: &Event, theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        // Handle input events
        let input_result = self.input.handle_event(event, theme);
        let button_result = self.calendar_button.handle_event(event, theme);

        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed: true,
                ..
            }) => {
                // Check button click
                if self.calendar_button.bounds().contains(*position) {
                    match self.mode {
                        DateTimePickerMode::Date => {
                            self.show_calendar = !self.show_calendar;
                            self.show_time_picker = false;
                        }
                        DateTimePickerMode::Time => {
                            self.show_calendar = false;
                            self.show_time_picker = !self.show_time_picker;
                        }
                        DateTimePickerMode::DateTime => {
                            self.show_calendar = !self.show_calendar;
                            self.show_time_picker = self.show_calendar;
                        }
                    }
                    return EventResult::Consumed;
                }

                // Check calendar click
                if self.show_calendar && self.calendar_rect.contains(*position) {
                    if self.handle_calendar_click(*position) {
                        return EventResult::Consumed;
                    }
                }

                // Check time picker click
                if self.show_time_picker && self.time_picker_rect.contains(*position) {
                    if self.handle_time_picker_click(*position) {
                        return EventResult::Consumed;
                    }
                }

                // Click outside popups closes them
                if !self.state.bounds.contains(*position)
                    && !self.calendar_rect.contains(*position)
                    && !self.time_picker_rect.contains(*position)
                {
                    self.show_calendar = false;
                    self.show_time_picker = false;
                    self.editing_hour = false;
                    self.editing_minute = false;
                }
            }
            Event::MouseMove(move_event) => {
                if self.show_calendar && self.calendar_rect.contains(move_event.position) {
                    // Update hovered day
                    let padding = 10;
                    let header_height = 30;
                    let cell_size = 30;
                    let calendar_y = self.calendar_rect.y() + header_height + padding;
                    let days_y = calendar_y + 20;
                    let first_day = Self::first_day_of_week(self.viewing_year, self.viewing_month);
                    let days_in_month = Self::days_in_month(self.viewing_year, self.viewing_month);

                    self.hovered_day = None;

                    for day in 1..=days_in_month {
                        let grid_pos = first_day + day - 1;
                        let row = grid_pos / 7;
                        let col = grid_pos % 7;

                        let x = self.calendar_rect.x() + padding + col as i32 * cell_size;
                        let y = days_y + row as i32 * cell_size;
                        let day_rect = Rect::new(x, y, cell_size, cell_size);

                        if day_rect.contains(move_event.position) {
                            self.hovered_day = Some(day);
                            break;
                        }
                    }

                    return EventResult::Consumed;
                }
            }
            Event::KeyPress(KeyPressEvent { key, .. }) => {
                if self.editing_hour || self.editing_minute {
                    match key {
                        Key::Character(ch) if ch.is_numeric() => {
                            if self.editing_hour {
                                if self.hour_input.len() < 2 {
                                    self.hour_input.push(*ch);
                                    self.update_time_from_inputs();
                                }
                            } else if self.editing_minute {
                                if self.minute_input.len() < 2 {
                                    self.minute_input.push(*ch);
                                    self.update_time_from_inputs();
                                }
                            }
                            return EventResult::Consumed;
                        }
                        Key::Backspace => {
                            if self.editing_hour {
                                self.hour_input.pop();
                                self.update_time_from_inputs();
                            } else if self.editing_minute {
                                self.minute_input.pop();
                                self.update_time_from_inputs();
                            }
                            return EventResult::Consumed;
                        }
                        Key::Tab => {
                            if self.editing_hour {
                                self.editing_hour = false;
                                self.editing_minute = true;
                            } else if self.editing_minute {
                                self.editing_minute = false;
                                self.editing_hour = true;
                            }
                            return EventResult::Consumed;
                        }
                        Key::Enter => {
                            self.editing_hour = false;
                            self.editing_minute = false;
                            self.show_time_picker = false;
                            if self.mode == DateTimePickerMode::Time {
                                self.show_calendar = false;
                            }
                            return EventResult::Consumed;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        if input_result == EventResult::Consumed || button_result == EventResult::Consumed {
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    }

    fn bounds(&self) -> Rect {
        self.state.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.state.bounds = bounds;
    }

    fn is_visible(&self) -> bool {
        self.state.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.state.visible = visible;
    }

    fn is_enabled(&self) -> bool {
        self.state.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.state.enabled = enabled;
    }

    fn is_focused(&self) -> bool {
        self.state.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.focused = focused;
        self.input.set_focused(focused);
    }

    fn can_focus(&self) -> bool {
        self.state.enabled && self.state.visible
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
