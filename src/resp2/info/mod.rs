pub enum InfoSection {
    REPLICATION,
}

#[allow(dead_code)]
impl InfoSection {
    pub fn from_str(section: &str) -> Self {
        match section.to_uppercase().as_str() {
            "REPLICATION" => InfoSection::REPLICATION,
            _ => panic!("Unknown INFO section: {}", section),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            InfoSection::REPLICATION => "REPLICATION".to_string(),
        }
    }
}
