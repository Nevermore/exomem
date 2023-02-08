/*
    Copyright 2019-2023 OÜ Nevermore <strom@nevermore.ee>

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

use ui::TaskManager;
use vault::{NodeKind, Provider, Vault};

const APP_NAME: &str = "exomem";

#[derive(Parser)]
#[command(bin_name = APP_NAME, name = APP_NAME, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all your files and directories.
    List {
        /// The directory to list the contents of.
        path: Option<String>,
    },
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
    /// Create a directory.
    Mkdir {
        /// The name of the directory to create.
        name: String,
    },
    /// Initialize state.
    Init {
        /// The name of the state file.
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let provider = Provider::new();

    if let Commands::Init { name } = &cli.command {
        TaskRunner::init(&provider, name);
        return;
    }

    let mut vault = Vault::open(&provider, "vault.db");
    let mut task_runner = TaskRunner::new(&mut vault);

    match &cli.command {
        Commands::List { path } => task_runner.list(path),
        Commands::Get { name } => task_runner.get(name),
        Commands::Put { name } => task_runner.put(name),
        Commands::Mkdir { name } => task_runner.create_directory(name),
        Commands::Init { .. } => unreachable!(),
    }
}

fn nice_node_kind(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Directory => "Directory",
        NodeKind::File => "File    .",
        NodeKind::Vault => "Vault   .",
    }
}

/// Runs requested tasks and prints output to console.
struct TaskRunner<'a> {
    task_manager: TaskManager<'a>,
}

impl<'a> TaskRunner<'a> {
    /// Create a new `TaskRunner` for running tasks.
    fn new(vault: &'a mut Vault<'a>) -> TaskRunner<'a> {
        TaskRunner {
            task_manager: TaskManager::new(vault),
        }
    }

    /// Print the list of entries in the directory.
    fn list(&mut self, path: &Option<String>) {
        let path = path.as_ref().map_or_else(|| "/", |path| path);
        println!("Listing {path}");
        let entries = self.task_manager.list(path);
        for (kind, name) in entries {
            println!("{}    {name}", nice_node_kind(kind));
        }
    }

    /// Get a specific file.
    fn get(&self, filename: &str) {
        match self.task_manager.get(filename) {
            Some(f) => println!("Indeed, we have: {}", f.name),
            None => println!("But we don't have: {filename}"),
        }
    }

    /// Put a specific file.
    fn put(&mut self, filename: &str) {
        match self.task_manager.put(filename) {
            Ok(f) => println!("Added: {}", f.name),
            Err(e) => println!("Failed to add: {e}"),
        }
    }

    fn init(provider: &Provider, name: &str) {
        TaskManager::init(provider, name);
    }

    /// Create a directory.
    fn create_directory(&mut self, path: &str) {
        self.task_manager.create_directory(path);
        /*
        match self.task_manager.create_directory(name) {
            Ok(f) => println!("Created: {}", f.name),
            Err(e) => println!("Failed to add: {e}"),
        }
        */
    }
}
