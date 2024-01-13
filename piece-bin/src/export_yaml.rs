use piece_lib::load_protos;

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    std::fs::create_dir_all("experimental/yaml")?;

    for (card, file) in cards {
        let file_path = std::path::Path::new(file.relative_path);
        let path = std::path::Path::new("experimental/yaml").join(
            file_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .chars()
                .next()
                .unwrap()
                .to_string(),
        );

        std::fs::create_dir_all(path.clone())?;

        std::fs::write(
            path.join(file_path.file_name().unwrap())
                .with_extension("yaml"),
            serde_yaml::to_string(&card)?,
        )?;
    }

    Ok(())
}
