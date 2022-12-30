use tracing::instrument;

#[instrument(ret)]
pub fn prioritise_work(request: &str) -> &str {
    request
        .split(',')
        .max_by(|toy1, toy2| parse_toy_quantity(toy1).cmp(&parse_toy_quantity(toy2)))
        .expect("Empty toy request sent")
}

fn parse_toy_quantity(toy: &str) -> u64 {
    toy.split_once('x')
        .expect("Toy parsing error")
        .0
        .parse()
        .expect("Expected number of toys")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toy_qtd() {
        assert_eq!(parse_toy_quantity("15x dog on a string"), 15);
    }

    #[test]
    fn test_prioritise_work() {
        let request = "10x toy car,15x dog on a string,4x inflatable motorcycle";
        let toy = prioritise_work(request);
        assert_eq!(toy, "15x dog on a string");
    }
}
