/*****************************************************************************
 *   Ledger App Ironfish Rust.
 *   (c) 2023 Ledger SAS.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *****************************************************************************/
use crate::accumulator::accumulate_data;
use crate::bolos::zlog_stack;
use crate::context::TxContext;
use crate::ironfish::constants::TX_HASH_LEN;
use crate::nvm::buffer::Buffer;
use crate::nvm::dkg_keys::DkgKeys;
use crate::nvm::get_and_clear_tx_hash;
use crate::utils::response::save_result;
use crate::AppSW;
use ironfish_frost::frost::keys::KeyPackage;
use ironfish_frost::frost::round1::SigningNonces;
use ironfish_frost::frost::round2;
use ironfish_frost::nonces::deterministic_signing_nonces;
use ironfish_frost::{frost::Randomizer, frost::SigningPackage};
use ledger_device_sdk::io::Comm;

#[inline(never)]
pub fn handler_dkg_sign(comm: &mut Comm, chunk: u8, ctx: &mut TxContext) -> Result<(), AppSW> {
    zlog_stack("start handler_dkg_sign\0");

    accumulate_data(comm, chunk, ctx)?;
    if !ctx.done {
        return Ok(());
    }

    let (frost_signing_package, randomizer, tx_hash) = parse_tx(&ctx.buffer)?;

    // By this point, the transaction should have already been reviewed.
    // Before proceeding, we need to ensure that the transaction was approved.
    // The transaction hash must be available and it should match the hash we received.
    let current_hash = get_and_clear_tx_hash().ok_or(AppSW::InvalidTxHash)?;

    if current_hash != tx_hash {
        zlog_stack("tx hash mismatch\0");
        return Err(AppSW::InvalidTxHash);
    }

    let key_package = DkgKeys.load_key_package()?;
    let nonces = generate_nonces(&key_package, tx_hash)?;

    zlog_stack("start signing\0");
    let signature = round2::sign(&frost_signing_package, &nonces, &key_package, randomizer)
        .map_err(|_| AppSW::TxSignFail)?;

    zlog_stack("unwrap sig result\0");
    let resp = signature.serialize();

    let total_chunks = save_result(ctx, resp.as_slice())?;
    comm.append(&total_chunks);
    Ok(())
}

#[inline(never)]
fn parse_tx(buffer: &Buffer) -> Result<(SigningPackage, Randomizer, &[u8]), AppSW> {
    zlog_stack("start parse_tx\0");

    let mut tx_pos = 0;

    let pk_randomness_len = buffer.get_u16(tx_pos)?;
    tx_pos += 2;

    let data = buffer.get_slice(tx_pos, tx_pos + pk_randomness_len)?;
    let randomizer = Randomizer::deserialize(data).map_err(|_| AppSW::InvalidRandomizer)?;
    tx_pos += pk_randomness_len;

    let frost_signing_package_len = buffer.get_u16(tx_pos)?;
    tx_pos += 2;

    let data = buffer.get_slice(tx_pos, tx_pos + frost_signing_package_len)?;
    let frost_signing_package =
        SigningPackage::deserialize(data).map_err(|_| AppSW::InvalidSigningPackage)?;
    tx_pos += frost_signing_package_len;

    let tx_hash = buffer.get_slice(tx_pos, tx_pos + TX_HASH_LEN)?;
    tx_pos += TX_HASH_LEN;

    if tx_pos != buffer.pos {
        return Err(AppSW::InvalidPayload);
    }

    Ok((frost_signing_package, randomizer, tx_hash))
}

#[inline(never)]
fn generate_nonces(key_package: &KeyPackage, tx_hash: &[u8]) -> Result<SigningNonces, AppSW> {
    let identities = DkgKeys.load_identities()?;

    Ok(deterministic_signing_nonces(
        key_package.signing_share(),
        tx_hash,
        &identities,
    ))
}
