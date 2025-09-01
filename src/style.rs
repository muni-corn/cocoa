use std::{fmt, process};

use console::Style;

pub const SUCCESS: Style = Style::new().green();
pub const WARNING: Style = Style::new().yellow();
pub const ERROR: Style = Style::new().red();
pub const INFO: Style = Style::new().blue();
pub const WHITE: Style = Style::new().white();
pub const DIM: Style = Style::new().dim();

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
        "{}  {}  {}",
        DIM.apply_to("│"),
        expression_style.apply_to(expression),
        msg_style.apply_to(msg)
    )
}

pub fn print_success_bold(msg: impl fmt::Display) {
    print_expression_message("✓", msg, SUCCESS, SUCCESS_BOLD);
}

pub fn print_warning_bold(msg: impl fmt::Display) {
    print_expression_message("♦", msg, WARNING, WARNING_BOLD);
}

pub fn print_error_bold(msg: impl fmt::Display) {
    print_expression_message("×", msg, ERROR, ERROR_BOLD);
}

pub fn print_info_bold(msg: impl fmt::Display) {
    print_expression_message("-", msg, INFO, INFO_BOLD);
}

pub fn print_success(msg: impl fmt::Display) {
    print_expression_message("-", msg, SUCCESS, SUCCESS);
}

pub fn print_warning(msg: impl fmt::Display) {
    print_expression_message("-", msg, WARNING, WARNING);
}

pub fn print_error(msg: impl fmt::Display) {
    print_expression_message("-", msg, ERROR, ERROR);
}

pub fn print_info(msg: impl fmt::Display) {
    print_expression_message("-", msg, INFO, INFO);
}

pub fn welcome(msg: impl fmt::Display) {
    println!("{} {}", DIM.apply_to("╭─"), WHITE_BOLD.apply_to(msg))
}

pub fn goodbye_with_death(code: i32) -> ! {
    println!("{} {}", DIM.apply_to("╰─"), ERROR_BOLD.apply_to("x~x"));
    process::exit(code)
}

pub fn goodbye_with_warning() {
    println!("{} {}", DIM.apply_to("╰─"), WARNING_BOLD.apply_to("o~o"));
}

pub fn goodbye_with_success() {
    println!("{} {}", DIM.apply_to("╰─"), SUCCESS_BOLD.apply_to("^u^"));
}
