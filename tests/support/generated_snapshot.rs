use json_schema_ast::SchemaDocument;
use jsoncompat_codegen::generate_dataclass_module_from_document;
use serde_json::Value;
use std::{fs, path::Path};

pub fn assert_split_snapshot(schema: &Value, output: &Path) {
    let module =
        generate_dataclass_module_from_document(&SchemaDocument::from_json(schema).unwrap())
            .unwrap();
    let implementation = module.private_file();
    let stem = output.file_stem().unwrap().to_str().unwrap();
    let companion = format!("_{stem}_generated");
    if std::env::var_os("JSONCOMPAT_UPDATE_DATACLASSES_FIXTURES").is_some() {
        fs::write(output, module.public_file(&companion)).unwrap();
        fs::write(
            output.with_file_name(format!("{companion}.py")),
            &implementation,
        )
        .unwrap();
    }
    assert_eq!(
        module.public_file(&companion),
        fs::read_to_string(output).unwrap().replace("\r\n", "\n"),
        "regenerate {} with jsoncompat codegen --output",
        output.display(),
    );
    assert_eq!(
        implementation,
        fs::read_to_string(output.with_file_name(format!("{companion}.py")))
            .unwrap()
            .replace("\r\n", "\n"),
    );
}
