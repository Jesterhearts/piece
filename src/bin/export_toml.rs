use piece::load_protos;

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    std::fs::create_dir_all("experimental/toml")?;

    for (card, file) in cards {
        std::fs::write(
            std::path::Path::new("experimental/toml")
                .join(file.path().file_name().unwrap())
                .with_extension("toml"),
            toml::to_string(&card)?,
        )?;
    }

    Ok(())
}
