use ratatui::style::Color;

// A dark, high-contrast palette tuned for Windows Terminal's truecolor support.
pub const ACCENT: Color = Color::Rgb(97, 175, 239); // primary focus / interactive accent (blue)
pub const FAVORITE: Color = Color::Rgb(229, 192, 123); // gold, for the favorite star
pub const SUCCESS: Color = Color::Rgb(152, 195, 121); // green, connected / trusted states
pub const DANGER: Color = Color::Rgb(224, 108, 117); // red, errors / destructive actions
pub const MUTED: Color = Color::Rgb(96, 103, 117); // dim gray, secondary/help text
pub const LABEL: Color = Color::Rgb(171, 178, 191); // soft gray, field labels
pub const TEXT: Color = Color::Rgb(224, 227, 232); // near-white, primary text/values
pub const BORDER: Color = Color::Rgb(64, 71, 88); // subtle default border
pub const SELECTED_BG: Color = Color::Rgb(40, 58, 82); // selected list row background
pub const BUTTON_ACTIVE_FG: Color = Color::Rgb(13, 16, 20); // text on a filled accent button
