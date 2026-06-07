use is_terminal::IsTerminal;

pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}
