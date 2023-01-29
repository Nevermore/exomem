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

use clap::{Parser, Subcommand};

use ui::Controller;

const APP_NAME: &str = "exomem";

#[derive(Parser)]
#[command(bin_name = APP_NAME, name = APP_NAME, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all your files.
    List,
    /// Get a file.
    Get {
        /// The file to get.
        name: String,
    },
    /// Put a file.
    Put {
        /// The file to put.
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let mut state = State::new();

    match &cli.command {
        Commands::List => state.list(),
        Commands::Get { name } => state.get(name),
        Commands::Put { name } => state.put(name),
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
            println!("Have file: {file}");
        }
    }

    fn get(&self, filename: &str) {
        match self.controller.get(filename) {
            Some(f) => println!("Indeed, we have: {}", f.name),
            None => println!("But we don't have: {filename}"),
        }
    }

    fn put(&mut self, filename: &str) {
        match self.controller.put(filename) {
            Ok(f) => println!("Added: {}", f.name),
            Err(e) => println!("Failed to add: {e}"),
        }
    }
}
