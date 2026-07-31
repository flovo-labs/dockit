//! Tests for `new`: casing normalization, the collision guard,
//! and byte-identical scaffolding — all against fixture repos.

mod common;

use tug::repo::Repo;
use tug::scaffold::{Casings, new_project};

#[test]
fn a_kebab_input_normalizes_into_all_three_casings() {
    let casings = Casings::of("my-app").unwrap();

    assert_eq!(casings.kebab, "my-app");
    assert_eq!(casings.constant, "MY_APP");
    assert_eq!(casings.class, "MyApp");
}

#[test]
fn underscores_whitespace_and_camel_case_transitions_are_word_boundaries() {
    for input in [
        "my_app",
        "my app",
        "myApp",
        "MyApp",
        "MY_APP",
        "My-code App",
    ] {
        let casings = Casings::of(input).unwrap();
        let expected_words = if input == "My-code App" { 3 } else { 2 };
        assert_eq!(
            casings.kebab.matches('-').count() + 1,
            expected_words,
            "{input}: {casings:?}"
        );
    }

    let casings = Casings::of("myGreatApp").unwrap();
    assert_eq!(casings.kebab, "my-great-app");
    assert_eq!(casings.constant, "MY_GREAT_APP");
    assert_eq!(casings.class, "MyGreatApp");

    // Runs of separators collapse; surrounding separators drop.
    let casings = Casings::of("  my--__app ").unwrap();
    assert_eq!(casings.kebab, "my-app");
}

#[test]
fn upper_case_input_folds_down_where_the_casing_calls_for_it() {
    let casings = Casings::of("MY_APP").unwrap();

    assert_eq!(casings.kebab, "my-app");
    assert_eq!(casings.constant, "MY_APP");
    assert_eq!(casings.class, "MyApp");
}

#[test]
fn a_name_with_no_words_is_an_error() {
    for input in ["", "  ", "-_-"] {
        let error = Casings::of(input).unwrap_err();
        assert!(error.to_string().contains("no words"), "{input:?}: {error}");
    }
}

// Golden files: the templates with the casings of `MyApp`
// applied, byte for byte. The expected literals are hand-rendered
// independently of the embedded templates.

const GOLDEN_COMPOSE: &str = "services:
  my-app:
    container_name: 'my-app'
    image: ''
    restart: 'unless-stopped'
    networks:
      - 'default-frontend'
    volumes:
      - '/etc/localtime:/etc/localtime:ro'
      - 'my-app-data:/data'
    labels:
      - 'traefik.enable=true'
      - 'traefik.http.routers.my-app.rule=Host(`${MY_APP_DOMAIN}`)'
      - 'traefik.http.routers.my-app.entrypoints=websecure'
      - 'traefik.http.routers.my-app.tls=true'
      - 'traefik.http.routers.my-app.tls.certresolver=letsencrypt'
      - 'traefik.http.routers.my-app.tls.domains[0].main=${MY_APP_DOMAIN}'
      - 'traefik.http.routers.my-app.middlewares=default@file'
      - 'traefik.http.services.my-app.loadbalancer.server.port=8080'

volumes:
  my-app-data:
    external: true

networks:
  default-frontend:
    external: true
";

const GOLDEN_LOCAL_COMPOSE: &str = "services:
  my-app:
    image: ''
";

const GOLDEN_PROJECT_JSON: &str = r#"{
  "name": "my-app",
  "implicitDependencies": [
    "networks",
    "traefik"
  ],
  "targets": {
    "create-volumes": {
      "options": {
        "volumes": [
          "my-app-data"
        ]
      }
    },
    "create-variables": {
      "options": {
        "variables": [
          {
            "name": "MY_APP_DOMAIN",
            "description": "Public domain name for MyApp.",
            "defaultValue": "my-app.${DEFAULT_DOMAIN_SUFFIX}"
          }
        ]
      }
    }
  }
}
"#;

const GOLDEN_ENV: &str = "
# *****************************************************************************
# MyApp
# *****************************************************************************

# Public domain name for MyApp.
# MY_APP_DOMAIN='my-app.my-domain.ch'
";

#[test]
fn new_scaffolds_the_four_template_files_byte_identically() {
    let root = common::fixture("basic");
    let repo = Repo::discover(&root).unwrap();

    new_project(&repo, "MyApp").unwrap();

    let dir = root.join("apps/my-app");
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    assert_eq!(read("docker-compose.yml"), GOLDEN_COMPOSE);
    assert_eq!(read("docker-compose.local.yml"), GOLDEN_LOCAL_COMPOSE);
    assert_eq!(read("project.json"), GOLDEN_PROJECT_JSON);
    assert_eq!(read(".env"), GOLDEN_ENV);
}

#[test]
fn a_name_colliding_after_normalization_fails_without_writing_anything() {
    let root = common::fixture("basic");
    let repo = Repo::discover(&root).unwrap();

    // The guard uses the normalized kebab name, not the raw input —
    // `Whoami` is not a project name, but it normalizes to the existing
    // `whoami`.
    let error = new_project(&repo, "Whoami").unwrap_err();

    assert!(error.to_string().contains("whoami"), "{error}");
    assert!(
        !root.join("apps/whoami/.env").exists(),
        "wrote into the existing project"
    );
}

#[test]
fn an_existing_target_directory_fails_without_writing_anything() {
    let root = common::fixture("basic");
    std::fs::create_dir_all(root.join("apps/my-app")).unwrap();
    std::fs::write(root.join("apps/my-app/notes.txt"), "precious").unwrap();
    let repo = Repo::discover(&root).unwrap();

    let error = new_project(&repo, "my-app").unwrap_err();

    assert!(error.to_string().contains("my-app"), "{error}");
    assert!(!root.join("apps/my-app/docker-compose.yml").exists());
}

#[test]
fn the_scaffolded_project_is_discoverable_and_well_formed() {
    let root = common::fixture("basic");
    let repo = Repo::discover(&root).unwrap();

    new_project(&repo, "my-app").unwrap();

    // Re-discovery parses the generated project.json and finds the declared
    // dependencies, volume, and variable.
    let repo = Repo::discover(&root).unwrap();
    let project = repo.project("my-app").unwrap();
    assert_eq!(project.implicit_dependencies, ["networks", "traefik"]);
    assert_eq!(project.volumes.len(), 1);
    assert_eq!(project.volumes[0].name, "my-app-data");
    assert_eq!(project.variables.len(), 1);
    assert_eq!(project.variables[0].name, "MY_APP_DOMAIN");
    assert_eq!(
        project.variables[0].default_value.as_deref(),
        Some("my-app.${DEFAULT_DOMAIN_SUFFIX}")
    );
    assert!(project.has_compose_file);
}
