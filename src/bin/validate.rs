use piece::{card::Card, load_protos};

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    for (card, _) in cards {
        let _: Card = (&card).try_into()?;
    }

    Ok(())
}
