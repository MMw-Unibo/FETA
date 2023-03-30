use std::fs::File;
use identity_iota::account::{Account, AccountBuilder, AutoSave, Error, IdentitySetup, MethodContent, Result};
use identity_iota::client::{Client, ClientBuilder, CredentialValidationOptions, CredentialValidator, FailFast, PresentationValidationOptions, Resolver, ResolverBuilder, SubjectHolderRelationship};
use identity_iota::core::{Duration, FromJson, json, OneOrMany, Timestamp, ToJson, Url};
use identity_iota::credential::{Credential, CredentialBuilder, Presentation, Subject};
use identity_iota::iota_core::{IotaDID, Network};
use identity_iota::account_storage::Stronghold;
use identity_iota::crypto::ProofOptions;
use identity_iota::did::DID;
use identity_iota::did::verifiable::VerifierOptions;
use std::path::PathBuf;
use std::sync::Arc;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use std::io::{BufRead, BufReader, Write};

pub fn write_did(did: &IotaDID) -> std::io::Result<()> {
    let mut output = File::create("/mnt/did.txt")?;
    write!(output, "{}", did)
}

pub fn read_did() -> std::io::Result<String> {
    let file = File::open("/mnt/did.txt").unwrap();
    let reader = BufReader::new(file);
    reader.lines().enumerate().next().unwrap().1
}

pub async fn create_client(network_name: String, url: String) -> Result<Client> {
    let network = Network::try_from_name(network_name)?;

    let client: Client = ClientBuilder::new()
        .network(network.clone())
        .primary_node(url.as_str(), None, None)?
        .build()
        .await?;
    Ok(client)
}

pub async fn create_builder(password: String, network_name: String, url: String) -> Result<AccountBuilder> {
    let stronghold_path: PathBuf = "/mnt/strong.hodl".into();
    let stronghold: Stronghold = Stronghold::new(&stronghold_path, password, None).await?;

    let network = Network::try_from_name(network_name)?;

    let builder: AccountBuilder = Account::builder()
        .autosave(AutoSave::Every)
        .autopublish(true)
        .storage(stronghold)
        .client_builder(
            ClientBuilder::new()
                .network(network.clone())
                .primary_node(url.as_str(), None, None)?,
        );
    Ok(builder)
}

pub async fn create_identity(builder: &mut AccountBuilder) -> Result<Account> {
    match builder.create_identity(IdentitySetup::default()).await {
        Ok(mut identity) => {
            identity
                .update_identity()
                .create_method()
                .content(MethodContent::GenerateEd25519)
                .fragment("issuerKey")
                .apply()
                .await?;
            Ok(identity)
        },
        Err(err) => {
            Err(err)
        }
    }
}

pub async fn load_identity(builder: &mut AccountBuilder, issuer_did: IotaDID) -> Result<Account> {
    match builder.load_identity(issuer_did).await {
        Ok(issuer) => Ok(issuer),
        Err(err) => Err(err),
    }
}

pub async fn crea_vc(issuer: &Account, holder: &IotaDID) -> Result<String> {
    let subject: Subject = Subject::from_json_value(json!({
    "id": holder,
    "name": "AccessoSmartContract",
  }))?;

    let mut credential: Credential = CredentialBuilder::default()
        .issuer(Url::parse(issuer.did().as_str())?)
        .type_("AccessoSC")
        .subject(subject)
        .build()?;

    issuer
        .sign("#issuerKey", &mut credential, ProofOptions::default())
        .await?;

    CredentialValidator::validate(
        &credential,
        &issuer.document(),
        &CredentialValidationOptions::default(),
        FailFast::FirstError,
    )
        .unwrap();

    let credential_json: String = credential.to_json()?;
    Ok(credential_json)
}

pub fn create_challenge() -> (String, Timestamp) {
    let mut gen = rand_chacha::ChaCha8Rng::from_entropy();
    let challenge = gen.next_u64().to_string();
    let expires: Timestamp = Timestamp::now_utc().checked_add(Duration::minutes(10)).unwrap();
    (challenge, expires)
}

pub async fn verify_vp(presentation_json: &String, issuer: &Account, challenge: (String, Timestamp)) -> Result<()> {
    let presentation: Presentation = Presentation::from_json(&presentation_json)?;

    let credential: Credential = match presentation.clone().verifiable_credential{
        OneOrMany::One(cre) => cre,
        OneOrMany::Many(_vec) => return Err(Error::IdentityNotFound),
    };
    CredentialValidator::validate(
        &credential,
        &issuer.document(),
        &CredentialValidationOptions::default(),
        FailFast::FirstError,
    )
        .unwrap();

    let presentation_verifier_options: VerifierOptions = VerifierOptions::new()
        .challenge(challenge.0.to_owned())
        .allow_expired(false);

    let credential_validation_options: CredentialValidationOptions = CredentialValidationOptions::default()
        .earliest_expiry_date(Timestamp::now_utc().checked_add(Duration::hours(10)).unwrap());

    let presentation_validation_options = PresentationValidationOptions::default()
        .presentation_verifier_options(presentation_verifier_options.clone())
        .shared_validation_options(credential_validation_options)
        .subject_holder_relationship(SubjectHolderRelationship::AlwaysSubject);

    let client: Client = match create_client(String::from("dev"), String::from("http://192.168.10.203:14265")).await {
        Ok(client) => client,
        Err(err) => return Err(err),
    };

    let resolver_builder: ResolverBuilder = ResolverBuilder::new();
    let resolver: Resolver = resolver_builder.client(Arc::from(client)).build().await?;
    //Resolver is created this way to connect to the private tanglem if I need to connect to mainnet i just need:
    //let resolver: Resolver = Resolver::new().await?;

    resolver
        .verify_presentation(
            &presentation,
            &presentation_validation_options,
            FailFast::FirstError,
            None,
            None,
        )
        .await?;

    Ok(())
}