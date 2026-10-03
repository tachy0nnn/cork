use clap::{ArgAction, Command, arg, command, value_parser};
use std::path::PathBuf;

pub fn build_cli() -> Command {
    command!()
        .version(crate::VERSION)
        .subcommand(Command::new("info").about("Print information"))
        .arg(arg!(-d --debug "Enable debug logging").action(ArgAction::SetTrue))
        .arg(
            arg!(-a --apk <PATH> "Use custom APK file")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
}
