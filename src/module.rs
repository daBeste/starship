use crate::context::Shell;
use crate::segment::{FillSegment, Segment};
use crate::utils::wrap_colorseq_for_shell;
use ansi_term::{ANSIString, ANSIStrings};
use std::fmt;
use std::time::Duration;

// List of all modules
// Default ordering is handled in configs/starship_root.rs
pub const ALL_MODULES: &[&str] = &[
    "aws",
    #[cfg(feature = "battery")]
    "battery",
    "character",
    "cmake",
    "cmd_duration",
    "cobol",
    "conda",
    "crystal",
    "dart",
    "deno",
    "directory",
    "docker_context",
    "dotnet",
    "elixir",
    "elm",
    "env_var",
    "erlang",
    "fill",
    "gcloud",
    "git_branch",
    "git_commit",
    "git_metrics",
    "git_state",
    "git_status",
    "golang",
    "helm",
    "hg_branch",
    "hostname",
    "java",
    "jobs",
    "julia",
    "kotlin",
    "kubernetes",
    "line_break",
    "lua",
    "memory_usage",
    "nim",
    "nix_shell",
    "nodejs",
    "ocaml",
    "openstack",
    "package",
    "perl",
    "php",
    "pulumi",
    "purescript",
    "python",
    "red",
    "rlang",
    "ruby",
    "rust",
    "scala",
    "shell",
    "shlvl",
    "singularity",
    "status",
    "swift",
    "terraform",
    "time",
    "username",
    "vagrant",
    "vcsh",
    "vlang",
    "zig",
];

/// A module is a collection of segments showing data for a single integration
/// (e.g. The git module shows the current git branch and status)
pub struct Module<'a> {
    /// The module's configuration map if available
    pub config: Option<&'a toml::Value>,

    /// The module's name, to be used in configuration and logging.
    name: String,

    /// The module's description
    description: String,

    /// The collection of segments that compose this module.
    pub segments: Vec<Segment>,

    /// the time it took to compute this module
    pub duration: Duration,
}

impl<'a> Module<'a> {
    /// Creates a module with no segments.
    pub fn new(name: &str, desc: &str, config: Option<&'a toml::Value>) -> Module<'a> {
        Module {
            config,
            name: name.to_string(),
            description: desc.to_string(),
            segments: Vec::new(),
            duration: Duration::default(),
        }
    }

    /// Set segments in module
    pub fn set_segments(&mut self, segments: Vec<Segment>) {
        self.segments = segments;
    }

    /// Get module's name
    pub fn get_name(&self) -> &String {
        &self.name
    }

    /// Get module's description
    pub fn get_description(&self) -> &String {
        &self.description
    }

    /// Whether a module has non-empty segments
    pub fn is_empty(&self) -> bool {
        self.segments
            .iter()
            // no trim: if we add spaces/linebreaks it's not "empty" as we change the final output
            .all(|segment| segment.value().is_empty())
    }

    /// Get values of the module's segments
    pub fn get_segments(&self) -> Vec<&str> {
        self.segments
            .iter()
            .map(|segment| segment.value())
            .collect()
    }

    /// Returns a vector of colored ANSIString elements to be later used with
    /// `ANSIStrings()` to optimize ANSI codes
    pub fn ansi_strings(&self) -> Vec<ANSIString> {
        self.ansi_strings_for_shell(Shell::Unknown, None)
    }

    pub fn ansi_strings_for_shell(&self, shell: Shell, width: Option<usize>) -> Vec<ANSIString> {
        let mut iter = self.segments.iter().peekable();
        let mut ansi_strings: Vec<ANSIString> = Vec::new();
        while iter.peek().is_some() {
            ansi_strings.extend(ansi_line(&mut iter, width));
        }

        match shell {
            Shell::Bash => ansi_strings_modified(ansi_strings, shell),
            Shell::Zsh => ansi_strings_modified(ansi_strings, shell),
            Shell::Tcsh => ansi_strings_modified(ansi_strings, shell),
            _ => ansi_strings,
        }
    }
}

impl<'a> fmt::Display for Module<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ansi_strings = self.ansi_strings();
        write!(f, "{}", ANSIStrings(&ansi_strings))
    }
}

fn ansi_strings_modified(ansi_strings: Vec<ANSIString>, shell: Shell) -> Vec<ANSIString> {
    ansi_strings
        .into_iter()
        .map(|ansi| {
            let wrapped = wrap_colorseq_for_shell(ansi.to_string(), shell);
            ANSIString::from(wrapped)
        })
        .collect::<Vec<ANSIString>>()
}

fn ansi_line<'a, I>(segments: &mut I, term_width: Option<usize>) -> Vec<ANSIString<'a>>
where
    I: Iterator<Item = &'a Segment>,
{
    let mut used = 0usize;
    let mut current: Vec<ANSIString> = Vec::new();
    let mut chunks: Vec<(Vec<ANSIString>, &FillSegment)> = Vec::new();

    for segment in segments {
        match segment {
            Segment::Fill(fs) => {
                chunks.push((current, fs));
                current = Vec::new();
            }
            _ => {
                used += segment.width_graphemes();
                current.push(segment.ansi_string());
            }
        }

        if let Segment::LineTerm = segment {
            break;
        }
    }

    if chunks.is_empty() {
        current
    } else {
        let fill_size = term_width
            .map(|tw| if tw > used { Some(tw - used) } else { None })
            .flatten()
            .map(|remaining| remaining / chunks.len());
        chunks
            .into_iter()
            .flat_map(|(strs, fill)| {
                strs.into_iter()
                    .chain(std::iter::once(fill.ansi_string(fill_size)))
            })
            .chain(current.into_iter())
            .collect::<Vec<ANSIString>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modules_is_in_alphabetical_order() {
        let mut sorted_modules: Vec<&str> = ALL_MODULES.iter().copied().collect();
        sorted_modules.sort_unstable();
        assert_eq!(sorted_modules.as_slice(), ALL_MODULES);
    }

    #[test]
    fn test_module_is_empty_with_no_segments() {
        let name = "unit_test";
        let desc = "This is a unit test";
        let module = Module {
            config: None,
            name: name.to_string(),
            description: desc.to_string(),
            segments: Vec::new(),
            duration: Duration::default(),
        };

        assert!(module.is_empty());
    }

    #[test]
    fn test_module_is_empty_with_all_empty_segments() {
        let name = "unit_test";
        let desc = "This is a unit test";
        let module = Module {
            config: None,
            name: name.to_string(),
            description: desc.to_string(),
            segments: Segment::from_text(None, ""),
            duration: Duration::default(),
        };

        assert!(module.is_empty());
    }

    #[test]
    fn test_module_is_not_empty_with_linebreak_only() {
        let name = "unit_test";
        let desc = "This is a unit test";
        let module = Module {
            config: None,
            name: name.to_string(),
            description: desc.to_string(),
            segments: Segment::from_text(None, "\n"),
            duration: Duration::default(),
        };

        assert!(!module.is_empty());
    }

    #[test]
    fn test_module_is_not_empty_with_space_only() {
        let name = "unit_test";
        let desc = "This is a unit test";
        let module = Module {
            config: None,
            name: name.to_string(),
            description: desc.to_string(),
            segments: Segment::from_text(None, " "),
            duration: Duration::default(),
        };

        assert!(!module.is_empty());
    }
}
