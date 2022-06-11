

#[post("/new_group", format = "application/json", data = "<group>")]
pub fn new_group(group: String) -> String {
    // Irgendwas machen
    format!("Gruppe \"{group}\" wurde empfangen!")
}