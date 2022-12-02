use clap::Parser;
use colored::Colorize;
use radix_engine::types::*;
use radix_engine_interface::core::*;
use radix_engine_interface::data::*;
use radix_engine_interface::model::NonFungibleAddress;
use radix_engine_interface::rule;
use transaction::builder::ManifestBuilder;
use transaction::model::Instruction;

use crate::resim::*;

#[scrypto(TypeId, Encode, Decode)]
struct EmptyStruct;

/// Create a badge with fixed supply
#[derive(Parser, Debug)]
pub struct NewOwnerBadge {
    /// The symbol
    #[clap(long)]
    symbol: Option<String>,

    /// The name
    #[clap(long)]
    name: Option<String>,

    /// The description
    #[clap(long)]
    description: Option<String>,

    /// The website URL
    #[clap(long)]
    url: Option<String>,

    /// The ICON url
    #[clap(long)]
    icon_url: Option<String>,

    /// The network to use when outputting manifest, [simulator | adapanet | nebunet | mainnet]
    #[clap(short, long)]
    network: Option<String>,

    /// Output a transaction manifest without execution
    #[clap(short, long)]
    manifest: Option<PathBuf>,

    /// The private keys used for signing, separated by comma
    #[clap(short, long)]
    signing_keys: Option<String>,

    /// Turn on tracing
    #[clap(short, long)]
    trace: bool,
}

impl NewOwnerBadge {
    pub fn run<O: std::io::Write>(&self, out: &mut O) -> Result<(), Error> {
        let default_account = get_default_account()?;
        let mut metadata = HashMap::new();
        if let Some(symbol) = self.symbol.clone() {
            metadata.insert("symbol".to_string(), symbol);
        }
        if let Some(name) = self.name.clone() {
            metadata.insert("name".to_string(), name);
        }
        if let Some(description) = self.description.clone() {
            metadata.insert("description".to_string(), description);
        }
        if let Some(url) = self.url.clone() {
            metadata.insert("url".to_string(), url);
        }
        if let Some(icon_url) = self.icon_url.clone() {
            metadata.insert("icon_url".to_string(), icon_url);
        };

        let mut resource_auth = HashMap::new();
        resource_auth.insert(ResourceMethodAuthKey::Withdraw, (rule!(allow_all), LOCKED));

        let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
            .lock_fee(FAUCET_COMPONENT, 100.into())
            .add_instruction(Instruction::CallNativeFunction {
                function_ident: NativeFunctionIdent {
                    blueprint_name: RESOURCE_MANAGER_BLUEPRINT.to_owned(),
                    function_name: ResourceManagerFunction::Create.to_string(),
                },
                args: args!(
                    ResourceType::NonFungible {
                        id_type: NonFungibleIdType::U32
                    },
                    metadata,
                    resource_auth,
                    Option::Some(MintParams::NonFungible {
                        entries: HashMap::from([(
                            NonFungibleId::U32(1),
                            (
                                scrypto_encode(&EmptyStruct).unwrap(),
                                scrypto_encode(&EmptyStruct).unwrap()
                            )
                        )])
                    })
                ),
            })
            .0
            .call_method(
                default_account,
                "deposit_batch",
                args!(Expression::entire_worktop()),
            )
            .build();
        let receipt = handle_manifest(
            manifest,
            &self.signing_keys,
            &self.network,
            &self.manifest,
            self.trace,
            false,
            false,
            out,
        )
        .unwrap()
        .unwrap();

        let resource_address = receipt
            .expect_commit()
            .entity_changes
            .new_resource_addresses[0];

        writeln!(
            out,
            "Owner badge: {}",
            NonFungibleAddress::new(resource_address, NonFungibleId::U32(1))
                .to_string()
                .green()
        )
        .map_err(Error::IOError)?;
        Ok(())
    }
}
