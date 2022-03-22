use anyhow::anyhow;
use anyhow::Result;

pub fn edit(front: &str, back: &str) -> Result<(String, String)> {
    let divider = "----------";
    let template = format!("{front}\n{divider}\n{back}\n");

    let output = scrawl::with(&template)?;

    output
        .split_once(divider)
        .map(|(front, back)| (front.trim().to_string(), back.trim().to_string()))
        .ok_or(anyhow!("Missing divider between front and back of card"))
        .and_then(|(front, back)| {
            if front.is_empty() {
                Err(anyhow!("Front of card can't be empty"))
            } else {
                Ok((front, back))
            }
        })
}
