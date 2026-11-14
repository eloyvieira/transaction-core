use transaction_core::account::Account;

fn main() {
    let account = Account::new(1);

    println!("Client: {}", account.client);
    println!("Available: {}", account.available);
    println!("Held: {}", account.held);
    println!("Total: {}", account.total());
    println!("Locked: {}", account.locked);
}