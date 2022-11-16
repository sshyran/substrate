// This file is part of Substrate.

// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{build_executor, state_machine_call_with_proof, SharedParams, State, LOG_TARGET};
use parity_scale_codec::{Decode, Encode};
use sc_executor::sp_wasm_interface::HostFunctions;
use sc_service::Configuration;
use sp_runtime::traits::{Block as BlockT, NumberFor};
use sp_weights::Weight;
use std::{fmt::Debug, str::FromStr};

/// Configurations of the [`Command::OnRuntimeUpgrade`].
#[derive(Debug, Clone, clap::Parser)]
pub struct OnRuntimeUpgradeCmd {
	/// The state type to use.
	#[command(subcommand)]
	pub state: State,

	/// Execute `try_state`, `pre_upgrade` and `post_upgrade` checks as well.
	///
	/// This will perform more checks, but it will also makes the reported PoV/Weight be
	/// inaccurate.
	#[clap(long)]
	pub checks: bool,
}

pub(crate) async fn on_runtime_upgrade<Block, HostFns>(
	shared: SharedParams,
	command: OnRuntimeUpgradeCmd,
	config: Configuration,
) -> sc_cli::Result<()>
where
	Block: BlockT + serde::de::DeserializeOwned,
	Block::Hash: FromStr,
	<Block::Hash as FromStr>::Err: Debug,
	Block::Header: serde::de::DeserializeOwned,
	NumberFor<Block>: FromStr,
	<NumberFor<Block> as FromStr>::Err: Debug,
	HostFns: HostFunctions,
{
	let executor = build_executor(&shared, &config);
	let ext = command.state.into_ext::<Block, HostFns>(&shared, &executor).await?;

	let (_, encoded_result) = state_machine_call_with_proof::<Block, HostFns>(
		&ext,
		&executor,
		"TryRuntime_on_runtime_upgrade",
		command.checks.encode().as_ref(),
		Default::default(), // we don't really need any extensions here.
	)?;

	let (weight, total_weight) = <(Weight, Weight) as Decode>::decode(&mut &*encoded_result)
		.map_err(|e| format!("failed to decode weight: {:?}", e))?;

	log::info!(
		target: LOG_TARGET,
		"TryRuntime_on_runtime_upgrade executed without errors. Consumed weight = ({} ps, {} byte), total weight = ({} ps, {} byte) ({:.2} %, {:.2} %).",
		weight.ref_time(), weight.proof_size(),
		total_weight.ref_time(), total_weight.proof_size(),
		(weight.ref_time() as f64 / total_weight.ref_time().max(1) as f64) * 100.0,
		(weight.proof_size() as f64 / total_weight.proof_size().max(1) as f64) * 100.0,
	);

	Ok(())
}
