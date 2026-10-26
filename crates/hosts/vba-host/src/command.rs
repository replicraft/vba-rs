//! Reading a command line the way the process launcher does.
//!
//! The policy decides about the *program* a `Shell` call starts, and the
//! launcher starts the program it reads from the same text. Both must read
//! the text identically, so the one reader lives here: white space separates
//! words, double quotes group, and nothing else is special (no globbing,
//! pipes, redirection or command separators).

/// Splits a command line into program and arguments.
pub fn split_command_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    out.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(cur);
    }
    out
}

/// The program a command line starts: its first word. `None` for a blank
/// line, an empty program name or a program name with a control character.
pub fn command_program(line: &str) -> Option<String> {
    let program = split_command_line(line).into_iter().next()?;
    if program.is_empty() || program.chars().any(char::is_control) {
        return None;
    }
    Some(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_lines_split_like_windows() {
        assert_eq!(split_command_line("a b  c"), ["a", "b", "c"]);
        assert_eq!(
            split_command_line("\"C:\\Program Files\\x.exe\" /k \"a b\""),
            ["C:\\Program Files\\x.exe", "/k", "a b"]
        );
        assert_eq!(
            split_command_line("echo hi; touch f"),
            ["echo", "hi;", "touch", "f"]
        );
        assert!(split_command_line("   ").is_empty());
        assert_eq!(split_command_line("\"\" x"), ["", "x"]);
    }

    #[test]
    fn the_program_is_the_first_word() {
        assert_eq!(command_program("notepad a.txt").as_deref(), Some("notepad"));
        assert_eq!(
            command_program("\"C:\\Program Files\\x.exe\" /k").as_deref(),
            Some("C:\\Program Files\\x.exe")
        );
        assert_eq!(command_program(""), None);
        assert_eq!(command_program("\"\" x"), None);
        assert_eq!(command_program("a\0b"), None);
        assert_eq!(command_program("a\nb c").as_deref(), Some("a"));
        // Quotes in the middle of a word group inside the word.
        assert_eq!(
            command_program("no\"tepad x\"").as_deref(),
            Some("notepad x")
        );
    }
}
