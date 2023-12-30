use piece::load_protos;

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    std::fs::create_dir_all("experimental/json")?;

    for (card, file) in cards {
        std::fs::write(
            std::path::Path::new("experimental/json")
                .join(file.path().file_name().unwrap())
                .with_extension("json"),
            serde_json::to_string_pretty(&card)?,
        )?;
    }

    Ok(())
}
