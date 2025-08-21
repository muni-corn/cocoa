use std::fmt;

use console::Style;

pub const SUCCESS: Style = Style::new().green();
pub const WARNING: Style = Style::new().yellow();
pub const ERROR: Style = Style::new().red();
pub const INFO: Style = Style::new().blue();
pub const WHITE: Style = Style::new().white();

pub const SUCCESS_BOLD: Style = SUCCESS.bold();
pub const WARNING_BOLD: Style = WARNING.bold();
pub const ERROR_BOLD: Style = ERROR.bold();
pub const INFO_BOLD: Style = INFO.bold();
pub const WHITE_BOLD: Style = WHITE.bold();

fn print_expression_message(
    expression: impl fmt::Display,
    msg: impl fmt::Display,
    expression_style: Style,
    msg_style: Style,
) {
    println!(
        "  {}  {}",
        expression_style.apply_to(expression),
        msg_style.apply_to(msg)
    )
}

pub fn print_success(msg: impl fmt::Display) {
    print_expression_message("^u^", msg, SUCCESS, SUCCESS_BOLD);
}

pub fn print_warning(msg: impl fmt::Display) {
    print_expression_message("o~o", msg, WARNING, WARNING_BOLD);
}

pub fn print_error(msg: impl fmt::Display) {
    print_expression_message("x~x", msg, ERROR, ERROR_BOLD);
}

pub fn print_info(msg: impl fmt::Display) {
    print_expression_message(" ~ ", msg, INFO, INFO);
}
