use crate::gitlab;

// TODO: implement
pub fn clone(token: String, url: String, dst: String) -> Result<(), String> {
    let gl = gitlab::Client::new(token, url)?;
    let projects = gl.get_projects().unwrap();

    dbg!(&projects);

    Ok(())
}
