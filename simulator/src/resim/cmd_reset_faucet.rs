use clap::Parser;
use radix_engine::system::bootstrap::DEFAULT_TESTING_FAUCET_SUPPLY;

use crate::resim::*;

/// Reset faucet balance
#[derive(Parser, Debug)]
pub struct ResetFaucet {}

#[derive(ScryptoDecode, Debug)]
struct Faucet {
    vault: Own,
    transactions: Own,
}

impl ResetFaucet {
    pub fn run<O: std::io::Write>(&self, out: &mut O) -> Result<(), Error> {
        let SimulatorEnvironment { mut db, .. } = SimulatorEnvironment::new()?;
        let reader = SystemDatabaseReader::new(&db);
        let data: Faucet = reader
            .read_typed_object_field(FAUCET_COMPONENT.as_node_id(), ModuleId::Main, 0)
            .unwrap();
        let vault_balance: NonFungibleVaultBalanceFieldPayload = reader
            .read_typed_object_field(
                &data.vault.0,
                ModuleId::Main,
                NonFungibleVaultField::Balance.into(),
            )
            .unwrap();

        let mut vault_balance = vault_balance.into_latest();
        let new_balance = *DEFAULT_TESTING_FAUCET_SUPPLY;
        writeln!(
            out,
            "Changing faucet balance from {} to {}",
            vault_balance.amount, new_balance
        )
        .map_err(Error::IOError)?;
        vault_balance.amount = new_balance;

        let mut writer = SystemDatabaseWriter::new(&mut db);
        writer
            .write_typed_object_field(
                &data.vault.0,
                ModuleId::Main,
                NonFungibleVaultField::Balance.into(),
                NonFungibleVaultBalanceFieldPayload::from_content_source(vault_balance),
            )
            .unwrap();

        Ok(())
    }
}
