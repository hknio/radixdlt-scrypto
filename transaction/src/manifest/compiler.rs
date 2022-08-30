use scrypto::address::Bech32Decoder;
use scrypto::core::NetworkDefinition;

use crate::manifest::*;
use crate::model::TransactionManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    LexerError(lexer::LexerError),
    ParserError(parser::ParserError),
    GeneratorError(generator::GeneratorError),
}

pub fn compile<T: BlobLoader>(
    s: &str,
    network: &NetworkDefinition,
    blob_loader: &mut T,
) -> Result<TransactionManifest, CompileError> {
    let bech32_decoder = Bech32Decoder::new(network);

    let tokens = lexer::tokenize(s).map_err(CompileError::LexerError)?;
    let instructions = parser::Parser::new(tokens)
        .parse_manifest()
        .map_err(CompileError::ParserError)?;
    generator::generate_manifest(&instructions, &bech32_decoder, blob_loader)
        .map_err(CompileError::GeneratorError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Instruction;
    use sbor::rust::collections::*;
    use sbor::rust::str::FromStr;
    use scrypto::address::Bech32Decoder;
    use scrypto::args;
    use scrypto::core::Blob;
    use scrypto::core::NetworkDefinition;
    use scrypto::crypto::*;
    use scrypto::math::*;
    use scrypto::resource::ResourceAddress;
    use scrypto::{prelude::Expression, resource::NonFungibleId};

    #[cfg(not(feature = "alloc"))]
    #[test]
    fn test_compile() {
        let bech32_decoder = Bech32Decoder::new(&NetworkDefinition::local_simulator());
        let mut blob_loader = FileBlobLoader::new("./examples/");
        let manifest = include_str!("../../examples/complex.rtm");
        let blob_hash = hash(include_bytes!("../../examples/package.data"));

        let component1 = bech32_decoder
            .validate_and_decode_component_address(
                "account_sim1q02r73u7nv47h80e30pc3q6ylsj7mgvparm3pnsm780qgsy064",
            )
            .unwrap();
        let component2 = bech32_decoder
            .validate_and_decode_component_address(
                "component_sim1q2f9vmyrmeladvz0ejfttcztqv3genlsgpu9vue83mcs835hum",
            )
            .unwrap();

        assert_eq!(
            crate::manifest::compile(
                manifest,
                &NetworkDefinition::local_simulator(),
                &mut blob_loader
            )
            .unwrap()
            .instructions,
            vec![
                Instruction::CallMethod {
                    component_address: component1,
                    method_name: "withdraw_by_amount".to_string(),
                    args: args!(
                        Decimal::from(5u32),
                        ResourceAddress::from_str(
                            "resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag"
                        )
                        .unwrap()
                    )
                },
                Instruction::TakeFromWorktopByAmount {
                    amount: Decimal::from(2),
                    resource_address: ResourceAddress::from_str(
                        "resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag"
                    )
                    .unwrap(),
                },
                Instruction::CallMethod {
                    component_address: component2,
                    method_name: "buy_gumball".to_string(),
                    args: args!(scrypto::resource::Bucket(512))
                },
                Instruction::AssertWorktopContainsByAmount {
                    amount: Decimal::from(3),
                    resource_address: ResourceAddress::from_str(
                        "resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag"
                    )
                    .unwrap(),
                },
                Instruction::AssertWorktopContains {
                    resource_address: ResourceAddress::from_str(
                        "resource_sim1qzhdk7tq68u8msj38r6v6yqa5myc64ejx3ud20zlh9gseqtux6"
                    )
                    .unwrap(),
                },
                Instruction::TakeFromWorktop {
                    resource_address: ResourceAddress::from_str(
                        "resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag"
                    )
                    .unwrap(),
                },
                Instruction::CreateProofFromBucket { bucket_id: 513 },
                Instruction::CloneProof { proof_id: 514 },
                Instruction::DropProof { proof_id: 514 },
                Instruction::DropProof { proof_id: 515 },
                Instruction::CallMethod {
                    component_address: component1,
                    method_name: "create_proof_by_amount".to_string(),
                    args: args!(
                        Decimal::from(5u32),
                        ResourceAddress::from_str(
                            "resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag"
                        )
                        .unwrap()
                    )
                },
                Instruction::PopFromAuthZone,
                Instruction::DropProof { proof_id: 516 },
                Instruction::ReturnToWorktop { bucket_id: 513 },
                Instruction::TakeFromWorktopByIds {
                    ids: BTreeSet::from([
                        NonFungibleId::from_str("0905000000").unwrap(),
                        NonFungibleId::from_str("0907000000").unwrap(),
                    ]),
                    resource_address: ResourceAddress::from_str(
                        "resource_sim1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqu57yag"
                    )
                    .unwrap()
                },
                Instruction::CallMethod {
                    component_address: component1,
                    method_name: "deposit_batch".into(),
                    args: args!(Expression("ENTIRE_WORKTOP".to_owned()))
                },
                Instruction::DropAllProofs,
                Instruction::CallMethod {
                    component_address: component2,
                    method_name: "complicated_method".to_string(),
                    args: args!(Decimal::from(1u32), PreciseDecimal::from(2u32))
                },
                Instruction::PublishPackage {
                    package: Blob(blob_hash)
                },
            ]
        );
    }
}
