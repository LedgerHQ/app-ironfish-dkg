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
use crate::app_ui::run_action::ui_run_action;
use crate::bolos::{zlog, zlog_stack};
use crate::context::TxContext;
use crate::crypto::chacha20poly::{compute_key, encrypt};
use crate::nvm::dkg_keys::DkgKeys;
use crate::utils::response::save_result;
use crate::AppSW;
use ledger_device_sdk::io::Comm;

#[inline(never)]
pub fn handler_dkg_backup_keys(comm: &mut Comm, ctx: &mut TxContext) -> Result<(), AppSW> {
    zlog("start handler_dkg_backup_keys\0");

    let data = DkgKeys.backup_keys()?;
    let key = compute_key();

    let resp = encrypt(&key, data)?;

    if !ui_run_action(&["Backup DKG Keys?"])? {
        return Err(AppSW::Deny);
    }

    let total_chunks = save_result(ctx, resp.as_slice())?;
    comm.append(&total_chunks);

    Ok(())
}