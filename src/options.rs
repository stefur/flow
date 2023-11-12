pub const HELP: &str = r#"flow
====

USAGE:
  flow [COMMAND] [ARGS]
FLAGS:
  -h, --help            Prints help information
COMMAND:
  cycle-tags            Takes two arguments. Direction (next or previous) and an optional number of available tags (Default: 9).
  toggle-tags           Toggle previous tags if selected tags already focused.
  focus-urgent-tags     Focus urgent tags on an output.
  focus-set-view-tags   Set view tags and then focus the tags.
"#;

#[derive(Debug)]
pub enum Arguments {
    Global {
        help: bool,
    },
    CycleTags {
        direction: String,
        n_tags: Option<u8>,
    },
    ToggleTags {
        to_tags: u32,
    },
    FocusUrgentTags,
    FocusSetViewTags {
        to_tags: u32,
    },
}

pub fn parse_args() -> Result<Arguments, Box<dyn std::error::Error>> {
    let mut pargs = pico_args::Arguments::from_env();

    match pargs.subcommand()?.as_deref() {
        Some("cycle-tags") => Ok(Arguments::CycleTags {
            direction: pargs.free_from_str()?,
            n_tags: pargs.opt_free_from_str()?,
        }),
        Some("toggle-tags") => Ok(Arguments::ToggleTags {
            to_tags: pargs.free_from_str()?,
        }),
        Some("focus-urgent-tags") => Ok(Arguments::FocusUrgentTags),
        Some("focus-set-view-tags") => Ok(Arguments::FocusSetViewTags {
            to_tags: pargs.free_from_str()?,
        }),
        Some(_) => Err("Unknown subcommand".into()),
        None => Ok(Arguments::Global {
            help: pargs.contains(["-h", "--help"]),
        }),
    }
}
