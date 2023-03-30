use std::fs::File;
use std::{fs, io};
use identity_iota::account::{Account, AccountBuilder, AutoSave, Error, IdentitySetup, MethodContent, Result};
use identity_iota::client::{Client as identityClient, ClientBuilder, CredentialValidationOptions, CredentialValidator, FailFast, Resolver, ResolverBuilder};
use identity_iota::core::{FromJson, OneOrMany, Timestamp, ToJson, Url};
use identity_iota::credential::{Credential, Presentation, PresentationBuilder};
use identity_iota::iota_core::{IotaDID, Network, MessageId};
use identity_iota::account_storage::{Stronghold};
use identity_iota::crypto::{GetSignature, GetSignatureMut, Proof, ProofOptions, SetSignature};
use ipfs_api::{IpfsApi, IpfsClient};
use futures::stream::TryStreamExt;
use std::path::PathBuf;
use std::io::{BufRead, BufReader, Write};

use identity_iota::did::verifiable::VerifierOptions;
use iota_client::{Client, Result as clientResult};
use std::sync::Arc;
use iota_client::bee_message::payload::Payload;
use sha2::{Sha256, Digest};


extern crate serde;

#[derive(serde::Serialize, serde::Deserialize)]
struct Signable {
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof: Option<Proof>,
}

impl Signable {
    pub fn new(data: String) -> Self {
        Self { data, proof: None }
    }
}

impl GetSignature for Signable {
    fn signature(&self) -> Option<&Proof> {
        self.proof.as_ref()
    }
}

impl GetSignatureMut for Signable {
    fn signature_mut(&mut self) -> Option<&mut Proof> {
        self.proof.as_mut()
    }
}

impl SetSignature for Signable {
    fn set_signature(&mut self, signature: identity_iota::crypto::Proof) {
        self.proof = Some(signature)
    }
}

pub fn write_did(did: &IotaDID) -> std::io::Result<()> {
    let mut output = File::create("/mnt/did.txt")?;
    write!(output, "{}", did)
}

pub fn write_vc(vc: &str) -> std::io::Result<()> {
    let mut output = File::create("/mnt/vc.txt")?;
    write!(output, "{}", vc)
}

pub fn write_content(content: String) -> std::io::Result<()> {
    let mut output = File::create("/mnt/ipfs_content.txt")?;
    write!(output, "{}", content)
}

pub fn read_did() -> std::io::Result<String> {
    let file = File::open("/mnt/did.txt").unwrap();
    let reader = BufReader::new(file);
    reader.lines().enumerate().next().unwrap().1
}

pub fn read_vc() -> std::io::Result<String> {
    let file = File::open("/mnt/vc.txt").unwrap();
    let reader = BufReader::new(file);
    reader.lines().enumerate().next().unwrap().1
}

pub async fn create_client_iota(network_name: String, url: String) -> clientResult<Client> {
    let client: Client = Client::builder()
        .with_network(&network_name)
        .with_primary_node(url.as_str(), None, None)?
        .finish()
        .await?;
    Ok(client)
}

pub async fn create_client_identity(network_name: String, url: String) -> Result<identityClient> {
    let network = Network::try_from_name(network_name)?;

    let client: identityClient = ClientBuilder::new()
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
                .fragment("SCKey")
                .apply()
                .await?;
            Ok(identity)
        },
        Err(err) => {
            Err(err)
        }
    }
}

pub async fn load_identity(builder: &mut AccountBuilder, did: IotaDID) -> Result<Account> {
    match builder.load_identity(did).await {
        Ok(issuer) => Ok(issuer),
        Err(err) => Err(err),
    }
}

pub async fn create_vp(credential_json: &String, holder: &Account, challenge: (String, Timestamp)) -> Result<String> {
    let credential: Credential = Credential::from_json(credential_json.as_str())?;

    let mut presentation: Presentation = PresentationBuilder::default()
        .holder(Url::parse(holder.did().as_ref())?)
        .credential(credential)
        .build()?;

    holder
        .sign(
            "#SCKey",
            &mut presentation,
            ProofOptions::new().challenge(challenge.0).expires(challenge.1),
        )
        .await?;

    let presentation_json: String = presentation.to_json()?;

    Ok(presentation_json)
}

pub async fn create_ipfs_content(user: &Account) -> Result<()> {
    let mut file = File::open("/mnt/simple.json").unwrap();
    let mut model = fs::read_to_string("/mnt/simple.json").unwrap();

    let mut hasher = Sha256::new();
    io::copy(&mut model.as_bytes(), &mut hasher).unwrap();
    let hash = hasher.finalize();
    let mut hex_hash = base16ct::lower::encode_string(&hash);
    let mut signed_hash = Signable::new(hex_hash.clone());
    user.sign("SCKey", &mut signed_hash, Default::default()).await?;
    let signable_serialized = serde_json::to_string(&signed_hash).unwrap();
    model.push('\n');
    model.push_str(&signable_serialized);

    write_content(model);
    Ok(())
}

pub async fn upload_to_tangle(user: &Account, cid: String, mut vc: String, index: &String) -> Result<()> {
    let client = create_client_iota(String::from("dev"), String::from("http://192.168.10.203:14265")).await.unwrap();

    vc.push('\n');
    vc.push_str(&cid);

    let mut vccid = Signable::new(vc.clone());
    user.sign("SCKey", &mut vccid, Default::default()).await?;

    let mut tag = String::from("IOTAFederatedLearning#");
    tag.push_str(&index);
    let content = serde_json::to_vec(&vccid).unwrap();

    let message = client
        .message()
        .with_index(tag)
        .with_data(content)
        .finish()
        .await;

    Ok(())
}

pub async fn get_models(client: &IpfsClient, index: &String, issuer_did: &IotaDID, clients_number: &usize) -> Result<Vec<String>> {
    let mut res = Vec::new();
    let iota_client = create_client_iota(String::from("dev"), String::from("http://192.168.10.203:14265")).await.unwrap();

    let identity_client: identityClient = match create_client_identity(String::from("dev"), String::from("http://192.168.10.203:14265")).await {
        Ok(client) => client,
        Err(err) => return Err(err),
    };

    let resolver_builder: ResolverBuilder = ResolverBuilder::new();
    let resolver: Resolver = resolver_builder.client(Arc::from(identity_client)).build().await.unwrap();

    let mut tag = String::from("IOTAFederatedLearning#");
    tag.push_str(&index);
    let mut message_ids_received: Vec<MessageId> = Vec::new();
    while message_ids_received.len() < *clients_number {
        let fetched_message_ids = iota_client.get_message().index(&tag).await.unwrap();
        for message_id in fetched_message_ids.iter() {
            if message_ids_received.contains(&message_id) {
                continue;
            } else {
                message_ids_received.push(message_id.clone());
            }
            let payload = iota_client.get_message().data(&message_id).await.unwrap().payload().to_owned().unwrap();

            if let Payload::Indexation(box_m) = payload {
                let data: Signable = serde_json::from_slice(box_m.as_ref().data()).unwrap();
                
                let mut lines = data.data.lines();
                
                let credential: Credential = Credential::from_json(lines.next().unwrap()).unwrap();
                let cid = lines.next().unwrap();

                let sub = match credential.clone().credential_subject {
                    OneOrMany::One(sub) => sub,
                    OneOrMany::Many(_vec) => return Err(Error::IdentityNotFound),
                };
                let user_did: IotaDID = IotaDID::parse(sub.id.unwrap().to_string()).unwrap();

                //Verify the signature on the data uploaded to the tangle
                let doc = resolver.resolve(&user_did).await.unwrap().document;
                let ver: bool = doc
                    .verify_data(&data, &VerifierOptions::default())
                    .is_ok();
                if ver {
                    let issuer_doc = resolver.resolve(&issuer_did).await.unwrap().document;
                    //Verify the VC contained in the data uploaded to the tangle
                    CredentialValidator::validate(
                        &credential,
                        &issuer_doc,
                        &CredentialValidationOptions::default(),
                        FailFast::FirstError,
                    ).unwrap();
                    let download = client
                        .cat(&cid)
                        .map_ok(|chunk| chunk.to_vec())
                        .try_concat()
                        .await;

                    let ipfs_content: String = String::from_utf8(download.unwrap()).unwrap();
                    let mut lines = ipfs_content.lines();
                    let mut model = lines.next().unwrap().to_string();

                    let sign = lines.next().unwrap().trim_end_matches(['\0', ' ']).to_string();
                    let signed_hash: Signable = serde_json::from_str(&sign).unwrap();
                    //Verify the signature on the hash
                    let ver: bool = doc
                        .verify_data(&signed_hash, &VerifierOptions::default())
                        .is_ok();
                    if ver {
                        //Verify the hash
                        let mut hasher = Sha256::new();
                        io::copy(&mut model.as_bytes(), &mut hasher).unwrap();
                        let hash = hasher.finalize();

                        let mut hex_hash = base16ct::lower::encode_string(&hash);
   
                        if hex_hash.eq(&signed_hash.data) {
                            res.push(model.to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(res)
}