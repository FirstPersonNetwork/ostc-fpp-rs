use ratatui::style::Color;

// CLI Color codes
/// Success state - Completed actions, valid inputs, positive feedback
pub const COLOR_SUCCESS: Color = Color::Rgb(61, 220, 132); // #3DDC84 - Android Green

///Using bright blue for professional, accessible appearance
pub const COLOR_BORDER: Color = Color::Rgb(97, 175, 239); // #61AFEF - Blue

/// Warning state - Warnings, cautions, important notices
pub const COLOR_WARNING: Color = Color::Rgb(255, 184, 108); // #FFB86C - Orange

/// Warning state - Accessible red for important warnings and cautions
pub const COLOR_WARNING_ACCESSIBLE_RED: Color = Color::Rgb(220, 100, 100); // #DC6464 - Accessible Red

/// Default text color
pub const COLOR_TEXT_DEFAULT: Color = Color::White;
