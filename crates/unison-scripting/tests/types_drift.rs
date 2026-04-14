//! Drift test: ensures `.d.ts` declarations and Lua VM bindings stay in sync.
//!
//! **Direction 1 (types -> VM):** Parse all `.d.ts` files, extract declared
//! globals / namespace names / instance methods, and assert they exist in a
//! fresh Lua VM with engine bindings registered.
//!
//! **Direction 2 (VM -> types):** Iterate `_G` in the VM, filter out Lua
//! stdlib, and assert every game-relevant global appears in the declarations.
//!
//! Runs headless, fast, no GPU.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use mlua::prelude::*;

// ===================================================================
// Tree-sitter helpers — extract declarations from `.d.ts` files
// ===================================================================

/// A declared global: either a `const` (table/constructor) or a `namespace`.
#[derive(Debug, Clone)]
struct DeclaredGlobal {
    name: String,
    kind: GlobalKind,
}

#[derive(Debug, Clone, PartialEq)]
enum GlobalKind {
    /// `declare const X: { ... }` or `declare const X: SomeType`
    Const,
    /// `declare namespace X { ... }`
    Namespace,
}

/// Method names declared on an interface (via `this: T` parameter pattern).
struct DeclaredInterface {
    /// Interface name (e.g. "World", "Color", "Rng", "UI")
    name: String,
    /// Method names declared in the interface body
    methods: Vec<String>,
}

/// Parse all `.d.ts` files in the given directory and extract:
/// - Top-level `declare const` names
/// - Top-level `declare namespace` names
/// - Interface method names (for interfaces that map to userdata)
fn parse_dts_declarations(types_dir: &Path) -> (Vec<DeclaredGlobal>, Vec<DeclaredInterface>) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .expect("failed to set TypeScript language");

    let mut globals = Vec::new();
    let mut interfaces = Vec::new();

    for entry in fs::read_dir(types_dir).expect("cannot read types/ dir") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ts") {
            continue;
        }

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let tree = parser
            .parse(&source, None)
            .unwrap_or_else(|| panic!("failed to parse {}", path.display()));
        let root = tree.root_node();

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() != "ambient_declaration" {
                continue;
            }

            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                match inner.kind() {
                    "lexical_declaration" => {
                        // `declare const X: ...`
                        let mut vc = inner.walk();
                        for vd in inner.children(&mut vc) {
                            if vd.kind() == "variable_declarator" {
                                if let Some(name_node) = vd.child_by_field_name("name") {
                                    globals.push(DeclaredGlobal {
                                        name: source[name_node.byte_range()].to_string(),
                                        kind: GlobalKind::Const,
                                    });
                                }
                            }
                        }
                    }
                    "internal_module" => {
                        // `declare namespace X { ... }`
                        if let Some(name_node) = inner.child_by_field_name("name") {
                            globals.push(DeclaredGlobal {
                                name: source[name_node.byte_range()].to_string(),
                                kind: GlobalKind::Namespace,
                            });
                        }
                    }
                    "interface_declaration" => {
                        // `declare interface X { ... }`
                        if let Some(name_node) = inner.child_by_field_name("name") {
                            let iface_name = source[name_node.byte_range()].to_string();
                            let methods = extract_interface_methods(&inner, &source);
                            if !methods.is_empty() {
                                interfaces.push(DeclaredInterface {
                                    name: iface_name,
                                    methods,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (globals, interfaces)
}

/// Extract method names from an interface body.
/// We look for `method_signature` nodes whose first parameter is `this: T`.
fn extract_interface_methods(iface_node: &tree_sitter::Node, source: &str) -> Vec<String> {
    let mut methods = Vec::new();

    if let Some(body) = iface_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            let method_name = match member.kind() {
                "method_signature" => {
                    member
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string())
                }
                "property_signature" => {
                    // Check if the type annotation is a function type (for `new:` etc.)
                    member
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string())
                }
                _ => None,
            };

            if let Some(name) = method_name {
                // Check if this method has a `this` parameter (instance method)
                let has_this = has_this_parameter(&member, source);
                if has_this {
                    methods.push(name);
                }
            }
        }
    }

    methods
}

/// Check if a method/property signature has a `this: X` first parameter.
fn has_this_parameter(node: &tree_sitter::Node, source: &str) -> bool {
    // For method_signature: check formal_parameters for `this`
    // For property_signature with function type: check the function type's params
    fn check_params(node: &tree_sitter::Node, source: &str) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "formal_parameters" {
                let mut pc = child.walk();
                for param in child.children(&mut pc) {
                    if param.kind() == "required_parameter" {
                        if let Some(pattern) = param.child_by_field_name("pattern") {
                            if &source[pattern.byte_range()] == "this" {
                                return true;
                            }
                        }
                    }
                }
            }
            // Recurse into function_type for property signatures
            if child.kind() == "type_annotation" || child.kind() == "function_type" {
                if check_params(&child, source) {
                    return true;
                }
            }
        }
        false
    }
    check_params(node, source)
}

/// Extract property (sub-table) names from the object type of `declare const unison: { ... }`.
/// Returns a map of property name → list of method names on that property's interface type.
fn extract_unison_sub_tables(types_dir: &Path) -> HashMap<String, Vec<String>> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .expect("failed to set TypeScript language");

    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    // Build an index of all interface declarations so we can resolve named types.
    let mut iface_index: HashMap<String, Vec<String>> = HashMap::new();

    for entry in fs::read_dir(types_dir).expect("cannot read types/ dir") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ts") {
            continue;
        }

        let source = fs::read_to_string(&path).unwrap();
        let tree = parser.parse(&source, None).unwrap();
        let root = tree.root_node();

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() != "ambient_declaration" {
                continue;
            }
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "interface_declaration" {
                    if let Some(name_node) = inner.child_by_field_name("name") {
                        let iface_name = source[name_node.byte_range()].to_string();
                        // Collect ALL method_signature and property_signature names
                        let methods = collect_all_interface_members(&inner, &source);
                        iface_index.insert(iface_name, methods);
                    }
                }
            }
        }
    }

    // Now find `declare const unison: { ... }` and extract sub-table method lists.
    for entry in fs::read_dir(types_dir).expect("cannot read types/ dir") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ts") {
            continue;
        }

        let source = fs::read_to_string(&path).unwrap();
        let tree = parser.parse(&source, None).unwrap();
        let root = tree.root_node();

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() != "ambient_declaration" {
                continue;
            }
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() != "lexical_declaration" {
                    continue;
                }
                let mut vc = inner.walk();
                for vd in inner.children(&mut vc) {
                    if vd.kind() != "variable_declarator" {
                        continue;
                    }
                    let name = match vd.child_by_field_name("name") {
                        Some(n) => source[n.byte_range()].to_string(),
                        None => continue,
                    };
                    if name != "unison" {
                        continue;
                    }
                    // Parse the type annotation of `unison: { ... }`
                    if let Some(type_ann) = vd.child_by_field_name("type") {
                        // Find object_type
                        let sub_props = collect_object_properties(&type_ann, &source, &iface_index);
                        for (prop_name, methods) in sub_props {
                            result.insert(prop_name, methods);
                        }
                    }
                }
            }
        }
    }

    result
}

/// Collect all property/method names from an interface body (without `this` filter).
fn collect_all_interface_members(iface_node: &tree_sitter::Node, source: &str) -> Vec<String> {
    let mut members = Vec::new();
    if let Some(body) = iface_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            if member.kind() == "method_signature" || member.kind() == "property_signature" {
                if let Some(name_node) = member.child_by_field_name("name") {
                    members.push(source[name_node.byte_range()].to_string());
                }
            }
        }
    }
    members
}

/// Parse an object_type node and return a map of property_name → method_names.
/// Uses `iface_index` to resolve named type references like `UnisonAssets`.
fn collect_object_properties(
    node: &tree_sitter::Node,
    source: &str,
    iface_index: &HashMap<String, Vec<String>>,
) -> Vec<(String, Vec<String>)> {
    let mut result = Vec::new();

    let mut walk_node = |n: &tree_sitter::Node| {
        if n.kind() == "object_type" {
            let mut oc = n.walk();
            for member in n.children(&mut oc) {
                if member.kind() == "property_signature" {
                    let prop_name = match member.child_by_field_name("name") {
                        Some(nn) => source[nn.byte_range()].to_string(),
                        None => continue,
                    };
                    // Get the type annotation of this property
                    if let Some(type_ann) = member.child_by_field_name("type") {
                        // Look for a type_identifier (named interface)
                        let type_name = find_type_identifier(&type_ann, source);
                        if let Some(tname) = type_name {
                            if let Some(methods) = iface_index.get(&tname) {
                                result.push((prop_name, methods.clone()));
                            }
                        }
                    }
                }
            }
        }
    };

    // Traverse into type_annotation → object_type
    traverse_for_object_type(node, source, iface_index, &mut walk_node);

    result
}

fn traverse_for_object_type(
    node: &tree_sitter::Node,
    source: &str,
    _iface_index: &HashMap<String, Vec<String>>,
    visitor: &mut dyn FnMut(&tree_sitter::Node),
) {
    visitor(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_for_object_type(&child, source, _iface_index, visitor);
    }
}

/// Find the first `type_identifier` (like `UnisonAssets`) in a type annotation subtree.
fn find_type_identifier(node: &tree_sitter::Node, source: &str) -> Option<String> {
    if node.kind() == "type_identifier" {
        return Some(source[node.byte_range()].to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_type_identifier(&child, source) {
            return Some(found);
        }
    }
    None
}

// ===================================================================
// Lua VM setup
// ===================================================================

/// Create a Lua VM with all engine bindings registered.
fn create_vm_with_bindings() -> Lua {
    let lua = Lua::new();
    unison_scripting::bindings::register_all(&lua).expect("register_all failed");
    lua
}

// ===================================================================
// Lua stdlib globals — filter these out for Direction 2
// ===================================================================

/// Globals that are part of Lua 5.4 stdlib (not engine-specific).
fn lua_stdlib_globals() -> BTreeSet<&'static str> {
    [
        // Lua 5.4 standard globals
        "_G", "_VERSION", "assert", "collectgarbage", "coroutine",
        "dofile", "error", "getmetatable", "io", "ipairs", "load",
        "loadfile", "math", "next", "os", "package", "pairs",
        "pcall", "print", "rawequal", "rawget", "rawlen", "rawset",
        "require", "select", "setmetatable", "string", "table",
        "tonumber", "tostring", "type", "utf8", "warn", "xpcall",
        // debug is stdlib — engine utilities are now under unison.debug
        "debug",
    ].into_iter().collect()
}

// ===================================================================
// Tests
// ===================================================================

/// Direction 1: every `declare const` and `declare namespace` in the .d.ts
/// files must exist as a global in the Lua VM.
#[test]
fn dts_globals_exist_in_vm() {
    let types_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("types");
    let (globals, _interfaces) = parse_dts_declarations(&types_dir);

    assert!(!globals.is_empty(), "parsed zero globals — is types/ empty?");

    let lua = create_vm_with_bindings();

    let mut missing = Vec::new();
    for g in &globals {
        let exists: bool = lua
            .load(format!(
                "return type({name}) ~= 'nil'",
                name = g.name
            ))
            .eval()
            .unwrap_or(false);

        if !exists {
            missing.push(format!("{} ({:?})", g.name, g.kind));
        }
    }

    assert!(
        missing.is_empty(),
        "Declared in .d.ts but missing from Lua VM:\n  {}",
        missing.join("\n  ")
    );
}

/// Direction 1 (cont'd): every method declared on `unison.*` sub-namespaces
/// (assets, renderer, input, scenes, events, UI, debug, math, World, Color, Rng)
/// must exist as a field at the correct path in the VM.
#[test]
fn dts_unison_submethods_exist_in_vm() {
    let types_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("types");
    let sub_tables = extract_unison_sub_tables(&types_dir);

    assert!(
        !sub_tables.is_empty(),
        "parsed zero unison sub-tables — is unison.d.ts present?"
    );

    let lua = create_vm_with_bindings();

    let mut missing = Vec::new();
    for (sub_name, methods) in &sub_tables {
        for method in methods {
            // Methods are either functions or (for constructors like World.new) checked as non-nil.
            let check = format!(
                "return unison.{sub}.{method} ~= nil",
                sub = sub_name,
                method = method,
            );
            let exists: bool = lua.load(&check).eval().unwrap_or(false);
            if !exists {
                missing.push(format!("unison.{sub_name}.{method}"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Methods declared in .d.ts but missing from Lua VM:\n  {}",
        missing.join("\n  ")
    );
}

/// Direction 1 (cont'd): instance methods on userdata types (World, Color,
/// Rng, UI) declared in .d.ts must be accessible on actual instances.
#[test]
fn dts_instance_methods_exist_on_userdata() {
    let types_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("types");
    let (_globals, interfaces) = parse_dts_declarations(&types_dir);

    let lua = create_vm_with_bindings();

    // Map interface names to Lua code that creates an instance.
    // Only test interfaces that correspond to actual userdata.
    let instance_creators: HashMap<&str, &str> = [
        ("World", "unison.World.new()"),
        ("Color", "unison.Color.hex(0xFF0000)"),
        ("Rng", "unison.Rng.new(42)"),
    ]
    .into_iter()
    .collect();

    let mut missing = Vec::new();

    for iface in &interfaces {
        let creator = match instance_creators.get(iface.name.as_str()) {
            Some(c) => c,
            None => continue, // skip interfaces we don't have instance creators for
        };

        for method in &iface.methods {
            let check = format!(
                r#"
                local inst = {creator}
                return type(inst.{method}) == "function"
                "#,
                creator = creator,
                method = method
            );
            let exists: bool = lua.load(&check).eval().unwrap_or(false);
            if !exists {
                missing.push(format!("{}:{}", iface.name, method));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Instance methods declared in .d.ts but missing from Lua VM:\n  {}",
        missing.join("\n  ")
    );
}

/// Direction 2: every non-stdlib global in the Lua VM should be declared
/// in the .d.ts files.
#[test]
fn vm_globals_declared_in_dts() {
    let types_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("types");
    let (globals, _interfaces) = parse_dts_declarations(&types_dir);

    let declared_names: BTreeSet<String> = globals.iter().map(|g| g.name.clone()).collect();
    let stdlib = lua_stdlib_globals();

    let lua = create_vm_with_bindings();

    // Collect all global names from _G
    let vm_globals: Vec<String> = lua
        .load(
            r#"
            local names = {}
            for k, _ in pairs(_G) do
                table.insert(names, k)
            end
            table.sort(names)
            return names
            "#,
        )
        .eval::<LuaTable>()
        .expect("failed to iterate _G")
        .sequence_values::<String>()
        .filter_map(|r| r.ok())
        .collect();

    let mut undeclared = Vec::new();
    for name in &vm_globals {
        // Skip Lua stdlib
        if stdlib.contains(name.as_str()) {
            continue;
        }
        // Skip internal/private globals (prefixed with __)
        if name.starts_with("__") {
            continue;
        }
        if !declared_names.contains(name) {
            undeclared.push(name.clone());
        }
    }

    assert!(
        undeclared.is_empty(),
        "Present in Lua VM but missing from .d.ts declarations:\n  {}",
        undeclared.join("\n  ")
    );
}

/// Direction 2 (cont'd): methods on `unison.*` sub-tables in the Lua VM
/// should be declared in the corresponding interface in the .d.ts.
#[test]
fn vm_unison_submethods_declared_in_dts() {
    let types_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("types");
    let declared_sub_tables = extract_unison_sub_tables(&types_dir);

    let lua = create_vm_with_bindings();

    // Sub-tables to verify: those we know are present on unison.*
    let sub_names = ["assets", "renderer", "input", "scenes", "events", "UI", "debug", "math", "World", "Color", "Rng"];

    let mut undeclared = Vec::new();

    for sub_name in &sub_names {
        // Get all keys from this sub-table in the VM
        let vm_keys: Vec<String> = lua
            .load(format!(
                r#"
                local names = {{}}
                local t = unison.{sub}
                if type(t) == "table" then
                    for k, _ in pairs(t) do
                        table.insert(names, tostring(k))
                    end
                end
                table.sort(names)
                return names
                "#,
                sub = sub_name
            ))
            .eval::<LuaTable>()
            .expect("failed to iterate sub-table")
            .sequence_values::<String>()
            .filter_map(|r| r.ok())
            .collect();

        let declared = declared_sub_tables
            .get(*sub_name)
            .cloned()
            .unwrap_or_default();
        let declared_set: BTreeSet<&str> = declared.iter().map(|s| s.as_str()).collect();

        for key in &vm_keys {
            if !declared_set.contains(key.as_str()) {
                undeclared.push(format!("unison.{sub_name}.{key}"));
            }
        }
    }

    assert!(
        undeclared.is_empty(),
        "Methods in Lua VM but missing from .d.ts declarations:\n  {}",
        undeclared.join("\n  ")
    );
}
