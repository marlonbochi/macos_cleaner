//! Local rules (FR-013): work offline, without an AI provider, and act as a safety floor for
//! AI answers (an item the rules know to be essential is never downgraded to Safe).
//!
//! Each rule is a small Strategy: a matcher plus the verdict it produces. Order matters —
//! the first matching rule wins, so specific rules come before generic ones.

use std::path::Path;

use super::prompt::display_path;
use super::verdict::Verdict;
use crate::domain::file_entry::{FileKind, FileRecord};
use crate::domain::risk::{Recommendation, RiskCategory, RiskLevel, SuggestedCommand};

/// Bump when the rules change: cached rule-based assessments are then recomputed.
pub const HEURISTIC_VERSION: &str = "heuristic-v1";

/// Normalised view of an item used by the matchers.
struct Item<'a> {
    /// Lower-case path with the home directory as `~`.
    path: String,
    ext: Option<String>,
    kind: FileKind,
    record: &'a FileRecord,
}

impl Item<'_> {
    fn under(&self, prefix: &str) -> bool {
        self.path == prefix || self.path.starts_with(&format!("{prefix}/"))
    }

    fn has_component(&self, name: &str) -> bool {
        self.path.split('/').any(|c| c == name)
    }

    fn ext_in(&self, list: &[&str]) -> bool {
        self.ext.as_deref().is_some_and(|e| list.contains(&e))
    }
}

struct Rule {
    matches: fn(&Item<'_>) -> bool,
    verdict: fn(&Item<'_>) -> Verdict,
}

#[derive(Debug, Clone)]
pub struct HeuristicAnalyzer {
    home: std::path::PathBuf,
}

impl HeuristicAnalyzer {
    pub fn new(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
        }
    }

    /// Always returns a verdict; `Unknown` when no rule applies.
    pub fn analyze(&self, record: &FileRecord) -> Verdict {
        let item = Item {
            path: display_path(&record.path, &self.home).to_lowercase(),
            ext: record.extension(),
            kind: record.kind,
            record,
        };
        RULES
            .iter()
            .find(|rule| (rule.matches)(&item))
            .map_or_else(|| unknown(&item), |rule| (rule.verdict)(&item))
            .sanitized()
    }
}

fn v(
    level: RiskLevel,
    category: RiskCategory,
    summary: &str,
    explanation: &str,
    recommendation: Recommendation,
) -> Verdict {
    Verdict {
        level,
        category,
        summary: summary.into(),
        explanation: explanation.into(),
        recommendation,
        steps: vec![],
        commands: vec![],
    }
}

fn cmd(command: &str, description: &str) -> SuggestedCommand {
    SuggestedCommand {
        command: command.into(),
        description: description.into(),
    }
}

const MEDIA: &[&str] = &[
    "mov", "mp4", "m4v", "mkv", "avi", "heic", "jpg", "jpeg", "png", "raw", "cr2", "nef", "arw",
    "dng", "mp3", "m4a", "wav", "aiff", "flac",
];
const DOCUMENTS: &[&str] = &[
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pages", "numbers", "key", "txt", "md",
    "psd", "ai", "sketch", "fig",
];
const ARCHIVES: &[&str] = &["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar"];
const INSTALLERS: &[&str] = &["dmg", "pkg", "mpkg", "iso", "xip", "ipsw"];
const VM_IMAGES: &[&str] = &["vmdk", "vdi", "qcow2", "vhd", "vhdx", "utm", "pvm", "img"];

static RULES: &[Rule] = &[
    Rule {
        matches: |i| {
            i.has_component("photos library.photoslibrary")
                || i.ext.as_deref() == Some("photoslibrary")
        },
        verdict: |_| {
            Verdict {
            steps: vec![
                "Fotos › Ajustes › iCloud › ative Fotos do iCloud".into(),
                "Escolha \"Otimizar Armazenamento do Mac\" para manter só versões reduzidas no disco".into(),
            ],
            ..v(RiskLevel::Dangerous, RiskCategory::Media,
                "Biblioteca do app Fotos.",
                "Contém suas fotos e vídeos e o banco de dados do Fotos. Apagar a pasta destrói a biblioteca.",
                Recommendation::MoveToCloudOrExternal)
        }
        },
    },
    Rule {
        matches: |i| i.under("~/library/mail") || i.under("~/library/messages"),
        verdict: |_| {
            v(
                RiskLevel::Dangerous,
                RiskCategory::AppData,
                "Dados do Mail ou do Mensagens.",
                "Guarda mensagens e anexos. Remova anexos pelo próprio app em vez de apagar a pasta.",
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/developer/xcode/deriveddata"),
        verdict: |_| Verdict {
            steps: vec!["Feche o Xcode antes de apagar".into()],
            commands: vec![cmd(
                "rm -rf ~/Library/Developer/Xcode/DerivedData/*",
                "Apaga os artefatos de build; o Xcode recria no próximo build",
            )],
            ..v(
                RiskLevel::Safe,
                RiskCategory::BuildArtifact,
                "Artefatos de build do Xcode (DerivedData).",
                "São recriados automaticamente na próxima compilação; só o primeiro build fica mais lento.",
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/developer/coresimulator"),
        verdict: |_| Verdict {
            steps: vec![
                "Xcode › Settings › Platforms: remova runtimes de iOS/watchOS que não usa".into(),
                "Window › Devices and Simulators: apague simuladores antigos".into(),
            ],
            commands: vec![
                cmd(
                    "xcrun simctl delete unavailable",
                    "Remove simuladores de runtimes que não estão mais instalados",
                ),
                cmd(
                    "xcrun simctl runtime list",
                    "Lista os runtimes de simulador instalados e seus tamanhos",
                ),
                cmd(
                    "xcrun simctl runtime delete <id>",
                    "Remove um runtime específico listado acima",
                ),
            ],
            ..v(
                RiskLevel::Caution,
                RiskCategory::VmImage,
                "Simuladores e runtimes do Xcode (CoreSimulator).",
                "Apagar a pasta à mão pode corromper o estado do Xcode e perde dados de apps instalados nos simuladores. Use as ferramentas do Xcode.",
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| {
            i.under("~/library/developer/xcode/ios devicesupport")
                || i.under("~/library/developer/xcode/watchos devicesupport")
        },
        verdict: |_| {
            v(
                RiskLevel::Safe,
                RiskCategory::Cache,
                "Símbolos de depuração de dispositivos (DeviceSupport).",
                "São baixados de novo quando você conecta o dispositivo ao Xcode.",
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/developer/xcode/archives"),
        verdict: |_| {
            v(
                RiskLevel::Caution,
                RiskCategory::Backup,
                "Archives de apps gerados pelo Xcode.",
                "Necessários para simbolicar crashes de versões publicadas. Apague só versões antigas, pelo Organizer.",
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| {
            i.under("~/library/containers/com.docker.docker") || i.path.ends_with("docker.raw")
        },
        verdict: |_| Verdict {
            steps: vec!["Docker Desktop › Settings › Resources: reduza o limite de disco".into()],
            commands: vec![
                cmd(
                    "docker system df",
                    "Mostra quanto espaço imagens, containers e volumes ocupam",
                ),
                cmd(
                    "docker system prune -a",
                    "Remove imagens, containers e redes não usados (pede confirmação)",
                ),
            ],
            ..v(
                RiskLevel::Caution,
                RiskCategory::VmImage,
                "Disco virtual do Docker Desktop.",
                "Contém imagens, containers e volumes. Apagar o arquivo perde volumes com dados; limpe pelo Docker.",
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/application support/mobilesync/backup"),
        verdict: |_| Verdict {
            steps: vec!["Finder › selecione o iPhone › Gerenciar Backups".into()],
            ..v(
                RiskLevel::Caution,
                RiskCategory::Backup,
                "Backups locais de iPhone/iPad.",
                "Podem ser o único backup do aparelho. Confirme que existe backup no iCloud antes de remover.",
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/mobile documents") || i.under("~/library/cloudstorage"),
        verdict: |_| {
            v(
                RiskLevel::Caution,
                RiskCategory::UserDocument,
                "Pasta sincronizada com a nuvem.",
                "Apagar aqui apaga também na nuvem. Para liberar espaço, use \"Remover Download\" no Finder e mantenha só na nuvem.",
                Recommendation::MoveToCloudOrExternal,
            )
        },
    },
    Rule {
        matches: |i| i.has_component(".trash"),
        verdict: |_| {
            v(
                RiskLevel::Safe,
                RiskCategory::Other,
                "Itens na Lixeira.",
                "Já foram descartados. Esvaziar a Lixeira libera o espaço.",
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| {
            i.has_component("node_modules")
                || i.has_component(".gradle")
                || (i.has_component("target") && i.kind != FileKind::File)
        },
        verdict: |_| {
            v(
                RiskLevel::Safe,
                RiskCategory::BuildArtifact,
                "Dependências ou artefatos de build de um projeto.",
                "Podem ser recriados pelo gerenciador de pacotes (npm install, cargo build, gradle).",
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| {
            i.under("~/library/caches") || i.under("/library/caches") || i.has_component("caches")
        },
        verdict: |_| Verdict {
            steps: vec!["Feche o app correspondente antes de apagar".into()],
            ..v(
                RiskLevel::Safe,
                RiskCategory::Cache,
                "Cache de aplicativo.",
                "Caches são recriados automaticamente; o app pode ficar mais lento na primeira abertura.",
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/logs") || i.under("/library/logs") || i.ext_in(&["log"]),
        verdict: |_| {
            v(
                RiskLevel::Safe,
                RiskCategory::Log,
                "Arquivos de log.",
                "Registros de diagnóstico; não são necessários para os apps funcionarem.",
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.kind == FileKind::Package && i.ext.as_deref() == Some("app"),
        verdict: |_| Verdict {
            steps: vec![
                "Arraste o app para o Lixo ou use o desinstalador do fabricante".into(),
                "Os dados do app ficam em ~/Library e podem ser revistos depois".into(),
            ],
            ..v(
                RiskLevel::Caution,
                RiskCategory::AppBundle,
                "Aplicativo instalado.",
                "Remover o app não apaga documentos, mas você precisará reinstalá-lo para usar de novo.",
                Recommendation::ReviewManually,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(INSTALLERS),
        verdict: |_| {
            v(
                RiskLevel::Safe,
                RiskCategory::Installer,
                "Instalador ou imagem de disco.",
                "Depois que o app foi instalado, o instalador geralmente pode ser baixado de novo.",
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(VM_IMAGES),
        verdict: |_| {
            v(
                RiskLevel::Caution,
                RiskCategory::VmImage,
                "Imagem de máquina virtual.",
                "Contém um sistema inteiro com possíveis dados. Exclua pela ferramenta de VM se não usar mais.",
                Recommendation::MoveToCloudOrExternal,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(ARCHIVES),
        verdict: |_| {
            v(
                RiskLevel::Caution,
                RiskCategory::Archive,
                "Arquivo compactado.",
                "Pode ser um backup ou o único exemplar de algo. Verifique o conteúdo antes.",
                Recommendation::ReviewManually,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(MEDIA),
        verdict: |_| {
            v(
                RiskLevel::Caution,
                RiskCategory::Media,
                "Arquivo de mídia pessoal.",
                "Fotos, vídeos e áudios costumam ser insubstituíveis. Considere mover para a nuvem ou um disco externo.",
                Recommendation::MoveToCloudOrExternal,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(DOCUMENTS),
        verdict: |_| {
            v(
                RiskLevel::Caution,
                RiskCategory::UserDocument,
                "Documento.",
                "Documentos podem ser únicos. Confirme se há cópia antes de apagar.",
                Recommendation::ReviewManually,
            )
        },
    },
];

fn unknown(item: &Item<'_>) -> Verdict {
    let what = match item.kind {
        FileKind::File => "Arquivo",
        FileKind::Directory => "Pasta",
        FileKind::Package => "Pacote",
    };
    v(
        RiskLevel::Unknown,
        RiskCategory::Other,
        &format!("{what} sem regra local conhecida ({}).", item.record.name()),
        "As regras locais não reconhecem este item. Configure um provedor de IA para uma análise detalhada, ou verifique manualmente.",
        Recommendation::ReviewManually,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::size::ByteSize;
    use std::path::PathBuf;

    fn analyze(path: &str, kind: FileKind) -> Verdict {
        let record = FileRecord {
            path: PathBuf::from(path),
            kind,
            size_on_disk: ByteSize::new(1),
            apparent_size: ByteSize::new(1),
            file_count: 1,
            created_at: None,
            modified_at: None,
            accessed_at: None,
            inode: 1,
            device: 1,
            is_hardlink_dup: false,
            is_protected: false,
        };
        HeuristicAnalyzer::new(Path::new("/Users/alice")).analyze(&record)
    }

    #[test]
    fn xcode_simulators_recommend_official_commands() {
        let v = analyze(
            "/Users/alice/Library/Developer/CoreSimulator/Devices",
            FileKind::Directory,
        );
        assert_eq!(v.level, RiskLevel::Caution);
        assert_eq!(v.recommendation, Recommendation::UseAppOrCommand);
        assert!(
            v.commands
                .iter()
                .any(|c| c.command == "xcrun simctl delete unavailable")
        );
    }

    #[test]
    fn photos_library_is_dangerous() {
        let v = analyze(
            "/Users/alice/Pictures/Photos Library.photoslibrary",
            FileKind::Package,
        );
        assert_eq!(v.level, RiskLevel::Dangerous);
        assert_eq!(v.recommendation, Recommendation::MoveToCloudOrExternal);
    }

    #[test]
    fn caches_and_build_artifacts_are_safe() {
        for (path, kind) in [
            (
                "/Users/alice/Library/Caches/com.spotify.client",
                FileKind::Directory,
            ),
            (
                "/Users/alice/Library/Developer/Xcode/DerivedData",
                FileKind::Directory,
            ),
            ("/Users/alice/code/app/node_modules", FileKind::Directory),
            ("/Users/alice/Downloads/Installer.dmg", FileKind::File),
            ("/Users/alice/.Trash/old.mov", FileKind::File),
        ] {
            assert_eq!(analyze(path, kind).level, RiskLevel::Safe, "{path}");
        }
    }

    #[test]
    fn personal_files_need_caution() {
        assert_eq!(
            analyze("/Users/alice/Movies/trip.MOV", FileKind::File).level,
            RiskLevel::Caution
        );
        assert_eq!(
            analyze("/Users/alice/Documents/tax.pdf", FileKind::File).level,
            RiskLevel::Caution
        );
        assert_eq!(
            analyze("/Users/alice/backup.zip", FileKind::File).level,
            RiskLevel::Caution
        );
    }

    #[test]
    fn a_file_named_target_is_not_a_build_folder() {
        assert_eq!(
            analyze("/Users/alice/code/target", FileKind::File).level,
            RiskLevel::Unknown
        );
        assert_eq!(
            analyze("/Users/alice/code/target", FileKind::Directory).level,
            RiskLevel::Safe
        );
    }

    #[test]
    fn unknown_items_point_to_ai_or_manual_review() {
        let v = analyze("/Users/alice/stuff.bin", FileKind::File);
        assert_eq!(v.level, RiskLevel::Unknown);
        assert_eq!(v.recommendation, Recommendation::ReviewManually);
        assert!(v.summary.contains("stuff.bin"));
    }
}
