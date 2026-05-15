// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::VecDeque;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    #[default]
    View,
    Log,
}

impl std::fmt::Display for AppTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppTab::View => write!(f, "View"),
            AppTab::Log => write!(f, "Log"),
        }
    }
}

#[derive(Default)]
pub struct AppState {
    pub log: VecDeque<String>,
    pub tab: AppTab,
}
