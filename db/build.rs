// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

extern crate syntex;
extern crate ethcore_ipc_codegen as codegen;

use std::env;
use std::path::Path;
use std::hash::Hash;

pub fn main() {
	let out_dir = env::var_os("OUT_DIR").unwrap();

	let rpc_codegen_entries = vec!["src/lib.rs.in", "src/lib.rs.in"];

	for entry in rpc_codegen_entries {
		// rpc pass
		if {
			let src = Path::new(entry);
			let dst = Path::new(&out_dir).join(enties);
			let mut registry = syntex::Registry::new();
			codegen::register(&mut registry);
			registry.expand("", &src, &dst).ok()
		}
		// binary serialization pass
		{
			let src = Path::new(&out_dir).join("lib.intermediate.rs.in");
			let dst = Path::new(&out_dir).join("lib.rs");
			let mut registry = syntex::Registry::new();
			codegen::register(&mut registry);
			registry.expand("", &src, &dst).unwrap();
		}
	}
}
