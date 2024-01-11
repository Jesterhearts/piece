use piece_lib::{card::Card, load_protos};

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    std::fs::create_dir_all("experimental/yaml")?;

    for (card, file) in cards {
        let _: Card = (&card).try_into()?;

        std::fs::write(
            std::path::Path::new("experimental/yaml")
                .join(file.path().file_name().unwrap())
                .with_extension("yaml"),
            serde_yaml::to_string(&card)?,
        )?;
    }

    Ok(())
}
