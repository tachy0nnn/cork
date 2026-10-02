use clap::{ArgAction, Command, arg, command};

pub fn build_cli() -> Command {
    command!()
        .version(crate::VERSION)
        .subcommand(Command::new("info").about("Print information"))
        .arg(arg!(-d --debug "Enable debug logging").action(ArgAction::SetTrue))
}
