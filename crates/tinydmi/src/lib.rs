pub mod parser;
pub mod prelude;

pub fn parse<S: AsRef<str>>(input: S) -> eyre::Result<prelude::Metadata> {
    prelude::Metadata::load(input)
}
