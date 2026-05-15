// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Logger {
    lines: VecDeque<String>,
}

impl Logger {
    pub fn log<S: ToString>(&mut self, text: S) {
        while self.lines.len() > 99 {
            self.lines.pop_front();
        }

        let text = text.to_string();
        println!("{text}");
        self.lines.push_back(text);
    }

    pub fn lines(&self) -> (&[String], &[String]) {
        self.lines.as_slices()
    }
}
