// Copyright [2022] [Mark Benvenuto]
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Result;

use super::types::CommonProcInfo;

pub fn get_procs() -> Result<Vec<CommonProcInfo>> {
    let mut procs = Vec::<CommonProcInfo>::new();

    for prc in procfs::process::all_processes()? {
        let cp = CommonProcInfo {
            pid: prc.pid,
            program: prc.stat.comm.clone(),
            cmdline: prc.cmdline().unwrap_or_default(),
            // env : prc.environ().unwrap_or_default(),
        };

        procs.push(cp);
    }

    Ok(procs)
}
