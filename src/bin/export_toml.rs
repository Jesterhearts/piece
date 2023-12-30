use piece::{card::Card, load_protos};

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    std::fs::create_dir_all("experimental/toml")?;

    for (card, file) in cards {
        let _: Card = (&card).try_into()?;

        std::fs::write(
            std::path::Path::new("experimental/toml")
                .join(file.path().file_name().unwrap())
                .with_extension("toml"),
            toml::to_string(&card)?,
        )?;
    }

    Ok(())
}
