/*
    Copyright 2019 OÜ Nevermore <strom@nevermore.ee>

    This file is part of exomem.

    Exomem is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

#[macro_use]
extern crate clap;

use clap::App;
use clap::Arg;
use clap::SubCommand;

use ui::Controller;

const APP_NAME: &str = "exomem";

fn main() {
    let matches = App::new(APP_NAME)
        .version(crate_version!())
        .subcommand(SubCommand::with_name("list").about("List all your files."))
        .subcommand(
            SubCommand::with_name("get").about("Get a file.").arg(
                Arg::with_name("file")
                    .help("The file to get.")
                    .index(1)
                    .required(true),
            ),
        )
        .subcommand(
            SubCommand::with_name("put").about("Put a file.").arg(
                Arg::with_name("file")
                    .help("The file to put.")
                    .index(1)
                    .required(true),
            ),
        )
        .get_matches();

    let mut state = State::new();

    match matches.subcommand() {
        ("list", Some(_)) => state.list(),
        ("get", Some(sub_m)) => state.get(sub_m.value_of("file").unwrap()),
        ("put", Some(sub_m)) => state.put(sub_m.value_of("file").unwrap()),
        ("", None) => (),
        _ => println!("Unknown command."),
    }
}

struct State {
    controller: Controller,
}

impl State {
    fn new() -> State {
        State {
            controller: Controller::new(),
        }
    }

    fn list(&self) {
        let files = self.controller.list_files();
        for file in files {
            println!("Have file: {}", file);
        }
    }

    fn get(&self, filename: &str) {
        match self.controller.get(filename) {
            Some(f) => println!("Indeed, we have: {}", f.name),
            None => println!("But we don't have: {}", filename),
        }
    }

    fn put(&mut self, filename: &str) {
        match self.controller.put(filename) {
            Ok(f) => println!("Added: {}", f.name),
            Err(e) => println!("Failed to add: {}", e),
        }
    }
}
