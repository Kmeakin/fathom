use fathom_minimal::{distillation, elaboration, surface, StringInterner};
use std::io::Read;
use std::path::PathBuf;
use structopt::StructOpt;
use typed_arena::Arena;

/// CLI for the programming language prototype.
#[derive(StructOpt)]
#[structopt(after_help = r#"EXAMPLES:

Using arguments

    fathom-minimal elab --surface-term=example-file
    fathom-minimal normalise --surface-term=example-file

Using pipes and redirects

    echo "(A : Type) -> (a : A) -> A" | fathom-minimal elab
    cat example-file | fathom-minimal elab
    fathom-minimal elab < example-file

Using heredocs

    fathom-minimal elab <<< "(A : Type) -> (a : A) -> A"

    fathom-minimal normalise <<EOF
        let id : (A : Type) -> (a : A) -> A
          = fun A => fun a => a;

        id Type Type
    EOF
"#)]
enum Options {
    /// Parse and elaborate a term, printing the elaborated term and type
    Elab(Args),
    /// Parse and elaborate a term, printing its normal form and type
    Normalise(Args),
    /// Parse and elaborate a term, printing its type
    Type(Args),
}

#[derive(StructOpt)]
struct Args {
    /// Path to a file containing the surface term (`-` to read from stdin)
    #[structopt(
        long = "surface-term",
        name = "FILE",
        default_value = "-",
        possible_values = &["-", "<path>"],
        parse(from_str),
    )]
    surface_term: Input,
}

enum Input {
    StdIn,
    Path(PathBuf),
}

impl From<&str> for Input {
    fn from(src: &str) -> Input {
        match src {
            "-" => Input::StdIn,
            _ => Input::Path(PathBuf::from(src)),
        }
    }
}

fn main() {
    let mut interner = StringInterner::new();
    let surface_arena = Arena::new();
    let core_arena = Arena::new();
    let pretty_arena = pretty::Arena::<()>::new();
    let mut context = elaboration::Context::new(&core_arena);

    match Options::from_args() {
        Options::Elab(Args { surface_term }) => {
            let surface_term = parse_term(&mut interner, &surface_arena, &surface_term);

            if let Some((term, r#type)) = context.synth(&surface_term) {
                if let Some(r#type) = context.readback(&core_arena, &r#type) {
                    use pretty::DocAllocator;

                    let mut context = distillation::Context::new(&surface_arena);
                    let term = context.check(&term);
                    let r#type = context.synth(&r#type);

                    let doc = (pretty_arena.nil())
                        .append(term.pretty(&interner, &pretty_arena))
                        .append(pretty_arena.space())
                        .append(pretty_arena.text(":"))
                        .group()
                        .append(pretty_arena.softline())
                        .append(r#type.pretty(&interner, &pretty_arena))
                        .group()
                        .into_doc();

                    println!("{}", doc.pretty(usize::MAX));
                }
            }
        }
        Options::Normalise(Args { surface_term }) => {
            let surface_term = parse_term(&mut interner, &surface_arena, &surface_term);

            if let Some((term, r#type)) = context.synth(&surface_term) {
                if let (Some(term), Some(r#type)) = (
                    context.normalize(&core_arena, &term),
                    context.readback(&core_arena, &r#type),
                ) {
                    use pretty::DocAllocator;

                    let mut context = distillation::Context::new(&surface_arena);
                    let term = context.check(&term);
                    let r#type = context.synth(&r#type);

                    let doc = (pretty_arena.nil())
                        .append(term.pretty(&interner, &pretty_arena))
                        .append(pretty_arena.space())
                        .append(pretty_arena.text(":"))
                        .group()
                        .append(pretty_arena.softline())
                        .append(r#type.pretty(&interner, &pretty_arena))
                        .group()
                        .into_doc();

                    println!("{}", doc.pretty(usize::MAX));
                }
            }
        }
        Options::Type(Args { surface_term }) => {
            let surface_term = parse_term(&mut interner, &surface_arena, &surface_term);

            if let Some((_, r#type)) = context.synth(&surface_term) {
                if let Some(r#type) = context.readback(&core_arena, &r#type) {
                    let mut context = distillation::Context::new(&surface_arena);
                    let r#type = context.synth(&r#type);

                    let doc = r#type.pretty(&interner, &pretty_arena).into_doc();

                    println!("{}", doc.pretty(usize::MAX));
                }
            }
        }
    }

    // Print diagnostics to stderr
    for message in context.drain_messages() {
        eprintln!("{}", message);
    }
}

fn parse_term<'arena>(
    interner: &mut StringInterner,
    arena: &'arena Arena<surface::Term<'arena>>,
    term: &Input,
) -> surface::Term<'arena> {
    // FIXME: error handling

    let mut source = String::new();
    match term {
        Input::StdIn => std::io::stdin().read_to_string(&mut source).unwrap(),
        Input::Path(path) => std::fs::File::open(path)
            .unwrap()
            .read_to_string(&mut source)
            .unwrap(),
    };

    surface::Term::parse(interner, arena, &source).unwrap()
}
