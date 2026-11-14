pub type ClientId = u16;
pub type Amount = i64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub client: ClientId,
    pub available: Amount,
    pub held: Amount,
    pub locked: bool,
}

impl Account {
    pub fn new(client: ClientId) -> Self {
        Self {
            client,
            available: 0,
            held: 0,
            locked: false,
        }
    }

    pub fn total(&self) -> Amount {
        self.available + self.held
    }
}
