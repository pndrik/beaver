// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use config::{Config, File, FileFormat};

use super::DEFAULTS;

#[test]
fn openai_is_disabled_by_default() {
    let config = Config::builder()
        .add_source(File::from_str(DEFAULTS, FileFormat::Yaml))
        .build()
        .unwrap();

    assert!(
        !config
            .get_bool("inference.providers.openai.enabled")
            .unwrap()
    );
    assert_eq!(
        config
            .get_string("inference.providers.openai.endpoint")
            .unwrap(),
        "https://api.openai.com/v1/"
    );
    assert_eq!(
        config
            .get_string("inference.providers.openai.apikey")
            .unwrap(),
        ""
    );
}
