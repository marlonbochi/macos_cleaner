//! Local rules (FR-013): work offline, without an AI provider, and act as a safety floor for
//! AI answers (an item the rules know to be essential is never downgraded to Safe).
//!
//! Each rule is a small Strategy: a matcher plus the verdict it produces. Order matters —
//! the first matching rule wins, so specific rules come before generic ones. Every text exists
//! in English and Brazilian Portuguese; the caller picks the language.

mod linux;
mod windows;

use std::path::Path;

use super::verdict::Verdict;
use crate::domain::file_entry::{FileKind, FileRecord};
use crate::domain::locale::Locale;
use crate::domain::platform::{Platform, normalized_path};
use crate::domain::risk::{Recommendation, RiskCategory, RiskLevel, SuggestedCommand};

/// Bump when the rules change: cached rule-based assessments are then recomputed.
pub const HEURISTIC_VERSION: &str = "heuristic-v3";

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
    verdict: fn(&Item<'_>, Locale) -> Verdict,
}

#[derive(Debug, Clone)]
pub struct HeuristicAnalyzer {
    home: String,
    platform: Platform,
}

impl HeuristicAnalyzer {
    /// Rules for the system this binary runs on.
    pub fn new(home: &Path) -> Self {
        Self::for_platform(home, Platform::current())
    }

    pub fn for_platform(home: &Path, platform: Platform) -> Self {
        Self {
            home: home.to_string_lossy().into_owned(),
            platform,
        }
    }

    /// Always returns a verdict; `Unknown` when no rule applies. Rules of the platform come
    /// first, then the rules shared by every system (macOS-only paths simply never match
    /// elsewhere).
    pub fn analyze(&self, record: &FileRecord, locale: Locale) -> Verdict {
        let item = Item {
            path: normalized_path(&record.path.to_string_lossy(), &self.home, self.platform),
            ext: record.extension(),
            kind: record.kind,
            record,
        };
        let platform_rules: &[Rule] = match self.platform {
            Platform::Windows => windows::RULES,
            Platform::Linux => linux::RULES,
            Platform::MacOs => &[],
        };
        platform_rules
            .iter()
            .chain(RULES)
            .find(|rule| (rule.matches)(&item))
            .map_or_else(
                || unknown(&item, locale),
                |rule| (rule.verdict)(&item, locale),
            )
            .sanitized()
    }
}

/// A localised text pair: `(english, portuguese)`.
type Text = (&'static str, &'static str);

fn v(
    l: Locale,
    level: RiskLevel,
    category: RiskCategory,
    summary: Text,
    explanation: Text,
    recommendation: Recommendation,
) -> Verdict {
    Verdict {
        level,
        category,
        summary: l.pick(summary.0, summary.1).to_owned(),
        explanation: l.pick(explanation.0, explanation.1).to_owned(),
        recommendation,
        steps: vec![],
        commands: vec![],
    }
}

fn steps(l: Locale, items: &[Text]) -> Vec<String> {
    items
        .iter()
        .map(|(en, pt)| l.pick(en, pt).to_owned())
        .collect()
}

fn cmd(l: Locale, command: &str, description: Text) -> SuggestedCommand {
    SuggestedCommand {
        command: command.into(),
        description: l.pick(description.0, description.1).to_owned(),
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
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[
                    (
                        "Photos › Settings › iCloud › turn on iCloud Photos",
                        "Fotos › Ajustes › iCloud › ative Fotos do iCloud",
                    ),
                    (
                        "Choose \"Optimize Mac Storage\" to keep only smaller versions on disk",
                        "Escolha \"Otimizar Armazenamento do Mac\" para manter só versões reduzidas no disco",
                    ),
                ],
            ),
            ..v(
                l,
                RiskLevel::Dangerous,
                RiskCategory::Media,
                ("Photos app library.", "Biblioteca do app Fotos."),
                (
                    "It holds your photos and videos and the Photos database. Deleting the folder destroys the library.",
                    "Contém suas fotos e vídeos e o banco de dados do Fotos. Apagar a pasta destrói a biblioteca.",
                ),
                Recommendation::MoveToCloudOrExternal,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/mail") || i.under("~/library/messages"),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Dangerous,
                RiskCategory::AppData,
                ("Mail or Messages data.", "Dados do Mail ou do Mensagens."),
                (
                    "It stores messages and attachments. Remove attachments from the app itself instead of deleting the folder.",
                    "Guarda mensagens e anexos. Remova anexos pelo próprio app em vez de apagar a pasta.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/developer/xcode/deriveddata"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Quit Xcode before deleting",
                    "Feche o Xcode antes de apagar",
                )],
            ),
            commands: vec![cmd(
                l,
                "rm -rf ~/Library/Developer/Xcode/DerivedData/*",
                (
                    "Deletes build artifacts; Xcode recreates them on the next build",
                    "Apaga os artefatos de build; o Xcode recria no próximo build",
                ),
            )],
            ..v(
                l,
                RiskLevel::Safe,
                RiskCategory::BuildArtifact,
                (
                    "Xcode build artifacts (DerivedData).",
                    "Artefatos de build do Xcode (DerivedData).",
                ),
                (
                    "They are recreated automatically on the next build; only the first build gets slower.",
                    "São recriados automaticamente na próxima compilação; só o primeiro build fica mais lento.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/developer/coresimulator"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[
                    (
                        "Xcode › Settings › Platforms: remove iOS/watchOS runtimes you no longer use",
                        "Xcode › Settings › Platforms: remova runtimes de iOS/watchOS que não usa",
                    ),
                    (
                        "Window › Devices and Simulators: delete old simulators",
                        "Window › Devices and Simulators: apague simuladores antigos",
                    ),
                ],
            ),
            commands: vec![
                cmd(
                    l,
                    "xcrun simctl delete unavailable",
                    (
                        "Removes simulators whose runtimes are no longer installed",
                        "Remove simuladores de runtimes que não estão mais instalados",
                    ),
                ),
                cmd(
                    l,
                    "xcrun simctl runtime list",
                    (
                        "Lists the installed simulator runtimes and their sizes",
                        "Lista os runtimes de simulador instalados e seus tamanhos",
                    ),
                ),
                cmd(
                    l,
                    "xcrun simctl runtime delete <id>",
                    (
                        "Removes a specific runtime listed above",
                        "Remove um runtime específico listado acima",
                    ),
                ),
            ],
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::VmImage,
                (
                    "Xcode simulators and runtimes (CoreSimulator).",
                    "Simuladores e runtimes do Xcode (CoreSimulator).",
                ),
                (
                    "Deleting the folder by hand can corrupt Xcode's state and loses the data of apps installed in the simulators. Use Xcode's tools.",
                    "Apagar a pasta à mão pode corromper o estado do Xcode e perde dados de apps instalados nos simuladores. Use as ferramentas do Xcode.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| {
            i.under("~/library/developer/xcode/ios devicesupport")
                || i.under("~/library/developer/xcode/watchos devicesupport")
        },
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::Cache,
                (
                    "Device debug symbols (DeviceSupport).",
                    "Símbolos de depuração de dispositivos (DeviceSupport).",
                ),
                (
                    "They are downloaded again when you connect the device to Xcode.",
                    "São baixados de novo quando você conecta o dispositivo ao Xcode.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/developer/xcode/archives"),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Caution,
                RiskCategory::Backup,
                (
                    "App archives produced by Xcode.",
                    "Archives de apps gerados pelo Xcode.",
                ),
                (
                    "Needed to symbolicate crashes from published versions. Delete only old versions, from the Organizer.",
                    "Necessários para simbolicar crashes de versões publicadas. Apague só versões antigas, pelo Organizer.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| {
            i.under("~/library/containers/com.docker.docker") || i.path.ends_with("docker.raw")
        },
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Docker Desktop › Settings › Resources: lower the disk limit",
                    "Docker Desktop › Settings › Resources: reduza o limite de disco",
                )],
            ),
            commands: vec![
                cmd(
                    l,
                    "docker system df",
                    (
                        "Shows how much space images, containers and volumes use",
                        "Mostra quanto espaço imagens, containers e volumes ocupam",
                    ),
                ),
                cmd(
                    l,
                    "docker system prune -a",
                    (
                        "Removes unused images, containers and networks (asks for confirmation)",
                        "Remove imagens, containers e redes não usados (pede confirmação)",
                    ),
                ),
            ],
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::VmImage,
                (
                    "Docker Desktop virtual disk.",
                    "Disco virtual do Docker Desktop.",
                ),
                (
                    "It holds images, containers and volumes. Deleting the file loses volumes with data; clean up through Docker.",
                    "Contém imagens, containers e volumes. Apagar o arquivo perde volumes com dados; limpe pelo Docker.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/application support/mobilesync/backup"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Finder › select the iPhone › Manage Backups",
                    "Finder › selecione o iPhone › Gerenciar Backups",
                )],
            ),
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::Backup,
                (
                    "Local iPhone/iPad backups.",
                    "Backups locais de iPhone/iPad.",
                ),
                (
                    "They may be the device's only backup. Make sure an iCloud backup exists before removing them.",
                    "Podem ser o único backup do aparelho. Confirme que existe backup no iCloud antes de remover.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/mobile documents") || i.under("~/library/cloudstorage"),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Caution,
                RiskCategory::UserDocument,
                (
                    "Folder synced with the cloud.",
                    "Pasta sincronizada com a nuvem.",
                ),
                (
                    "Deleting here also deletes it in the cloud. To free space, use \"Remove Download\" in Finder and keep it only in the cloud.",
                    "Apagar aqui apaga também na nuvem. Para liberar espaço, use \"Remover Download\" no Finder e mantenha só na nuvem.",
                ),
                Recommendation::MoveToCloudOrExternal,
            )
        },
    },
    Rule {
        matches: |i| i.has_component(".trash"),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::Other,
                ("Items in the Trash.", "Itens na Lixeira."),
                (
                    "They were already discarded. Emptying the Trash frees the space.",
                    "Já foram descartados. Esvaziar a Lixeira libera o espaço.",
                ),
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
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::BuildArtifact,
                (
                    "A project's dependencies or build artifacts.",
                    "Dependências ou artefatos de build de um projeto.",
                ),
                (
                    "They can be recreated by the package manager (npm install, cargo build, gradle).",
                    "Podem ser recriados pelo gerenciador de pacotes (npm install, cargo build, gradle).",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| {
            i.under("~/library/caches") || i.under("/library/caches") || i.has_component("caches")
        },
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Quit the related app before deleting",
                    "Feche o app correspondente antes de apagar",
                )],
            ),
            ..v(
                l,
                RiskLevel::Safe,
                RiskCategory::Cache,
                ("Application cache.", "Cache de aplicativo."),
                (
                    "Caches are recreated automatically; the app may be slower the first time it opens.",
                    "Caches são recriados automaticamente; o app pode ficar mais lento na primeira abertura.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/library/logs") || i.under("/library/logs") || i.ext_in(&["log"]),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::Log,
                ("Log files.", "Arquivos de log."),
                (
                    "Diagnostic records; apps do not need them to work.",
                    "Registros de diagnóstico; não são necessários para os apps funcionarem.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.kind == FileKind::Package && i.ext.as_deref() == Some("app"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[
                    (
                        "Drag the app to the Trash or use the vendor's uninstaller",
                        "Arraste o app para o Lixo ou use o desinstalador do fabricante",
                    ),
                    (
                        "The app's data stays in ~/Library and can be reviewed afterwards",
                        "Os dados do app ficam em ~/Library e podem ser revistos depois",
                    ),
                ],
            ),
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::AppBundle,
                ("Installed application.", "Aplicativo instalado."),
                (
                    "Removing the app does not delete documents, but you will need to reinstall it to use it again.",
                    "Remover o app não apaga documentos, mas você precisará reinstalá-lo para usar de novo.",
                ),
                Recommendation::ReviewManually,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(INSTALLERS),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::Installer,
                ("Installer or disk image.", "Instalador ou imagem de disco."),
                (
                    "Once the app is installed, the installer can usually be downloaded again.",
                    "Depois que o app foi instalado, o instalador geralmente pode ser baixado de novo.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(VM_IMAGES),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Caution,
                RiskCategory::VmImage,
                ("Virtual machine image.", "Imagem de máquina virtual."),
                (
                    "It contains a whole system with possible data. Delete it from the VM tool if you no longer use it.",
                    "Contém um sistema inteiro com possíveis dados. Exclua pela ferramenta de VM se não usar mais.",
                ),
                Recommendation::MoveToCloudOrExternal,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(ARCHIVES),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Caution,
                RiskCategory::Archive,
                ("Compressed archive.", "Arquivo compactado."),
                (
                    "It may be a backup or the only copy of something. Check its contents first.",
                    "Pode ser um backup ou o único exemplar de algo. Verifique o conteúdo antes.",
                ),
                Recommendation::ReviewManually,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(MEDIA),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Caution,
                RiskCategory::Media,
                ("Personal media file.", "Arquivo de mídia pessoal."),
                (
                    "Photos, videos and audio are often irreplaceable. Consider moving them to the cloud or an external drive.",
                    "Fotos, vídeos e áudios costumam ser insubstituíveis. Considere mover para a nuvem ou um disco externo.",
                ),
                Recommendation::MoveToCloudOrExternal,
            )
        },
    },
    Rule {
        matches: |i| i.ext_in(DOCUMENTS),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Caution,
                RiskCategory::UserDocument,
                ("Document.", "Documento."),
                (
                    "Documents may be unique. Make sure there is a copy before deleting.",
                    "Documentos podem ser únicos. Confirme se há cópia antes de apagar.",
                ),
                Recommendation::ReviewManually,
            )
        },
    },
];

fn unknown(item: &Item<'_>, l: Locale) -> Verdict {
    let name = item.record.name();
    let summary = match (item.kind, l) {
        (FileKind::File, Locale::EnUs) => format!("File with no known local rule ({name})."),
        (FileKind::Directory, Locale::EnUs) => format!("Folder with no known local rule ({name})."),
        (FileKind::Package, Locale::EnUs) => format!("Package with no known local rule ({name})."),
        (FileKind::File, Locale::PtBr) => format!("Arquivo sem regra local conhecida ({name})."),
        (FileKind::Directory, Locale::PtBr) => format!("Pasta sem regra local conhecida ({name})."),
        (FileKind::Package, Locale::PtBr) => format!("Pacote sem regra local conhecida ({name})."),
    };
    Verdict {
        summary,
        ..v(
            l,
            RiskLevel::Unknown,
            RiskCategory::Other,
            ("", ""),
            (
                "The local rules do not recognise this item. Configure an AI provider for a detailed analysis, or check it manually.",
                "As regras locais não reconhecem este item. Configure um provedor de IA para uma análise detalhada, ou verifique manualmente.",
            ),
            Recommendation::ReviewManually,
        )
    }
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
        HeuristicAnalyzer::new(Path::new("/Users/alice")).analyze(&record, Locale::PtBr)
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

    fn analyze_on(platform: Platform, home: &str, path: &str, kind: FileKind) -> Verdict {
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
        HeuristicAnalyzer::for_platform(Path::new(home), platform).analyze(&record, Locale::EnUs)
    }

    #[test]
    fn windows_rules_recognise_system_files_and_caches() {
        let win = |path: &str, kind| analyze_on(Platform::Windows, r"C:\Users\Bob", path, kind);

        let temp = win(r"C:\Users\Bob\AppData\Local\Temp", FileKind::Directory);
        assert_eq!((temp.level, temp.category), (RiskLevel::Safe, RiskCategory::Cache));

        let hiber = win(r"C:\hiberfil.sys", FileKind::File);
        assert!(hiber.commands.iter().any(|c| c.command == "powercfg /hibernate off"));

        assert_eq!(win(r"C:\pagefile.sys", FileKind::File).level, RiskLevel::Dangerous);
        assert_eq!(win(r"D:\Windows.old", FileKind::Directory).category, RiskCategory::Backup);
        assert_eq!(
            win(r"C:\Program Files\Some App", FileKind::Directory).category,
            RiskCategory::AppBundle
        );
        assert_eq!(
            win(r"C:\Program Files\Some App\data", FileKind::Directory).category,
            RiskCategory::Other,
            "only the program folder itself is an app"
        );
        assert_eq!(
            win(r"C:\Users\Bob\Downloads\setup.EXE", FileKind::File).category,
            RiskCategory::Installer
        );
        assert_eq!(
            win(r"C:\Program Files (x86)\Steam\steamapps\common\Game", FileKind::Directory)
                .category,
            RiskCategory::Game
        );
        assert_eq!(
            win(
                r"C:\Users\Bob\AppData\Local\Google\Chrome\User Data\Default\Cache",
                FileKind::Directory
            )
            .level,
            RiskLevel::Safe
        );
        assert_eq!(
            win(r"C:\Users\Bob\Videos\trip.mp4", FileKind::File).category,
            RiskCategory::Media,
            "shared rules still apply"
        );
    }

    #[test]
    fn linux_rules_recommend_the_system_tools() {
        let linux = |path: &str, kind| analyze_on(Platform::Linux, "/home/bob", path, kind);

        let journal = linux("/var/log/journal", FileKind::Directory);
        assert!(journal.commands.iter().any(|c| c.command.contains("journalctl --vacuum-size")));
        assert!(
            linux("/var/cache/apt/archives", FileKind::Directory)
                .commands
                .iter()
                .any(|c| c.command == "sudo apt clean")
        );
        assert_eq!(linux("/home/bob/.cache/mozilla", FileKind::Directory).level, RiskLevel::Safe);
        assert_eq!(
            linux("/home/bob/.local/share/Trash/files/x.iso", FileKind::File).level,
            RiskLevel::Safe
        );
        assert_eq!(linux("/opt/google", FileKind::Directory).category, RiskCategory::AppBundle);
        assert_eq!(
            linux("/home/bob/Downloads/app.deb", FileKind::File).category,
            RiskCategory::Installer
        );
        assert_eq!(
            linux("/home/bob/Library/Caches/x", FileKind::Directory).level,
            RiskLevel::Safe,
            "generic cache rule still applies"
        );
    }

    #[test]
    fn windows_paths_are_never_matched_by_unix_rules() {
        let v = analyze_on(Platform::MacOs, "/Users/bob", "/var/cache/apt", FileKind::Directory);
        assert_ne!(v.commands.first().map(|c| c.command.as_str()), Some("sudo apt clean"));
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
    fn texts_follow_the_requested_language() {
        let record = FileRecord {
            path: PathBuf::from("/Users/alice/Library/Developer/CoreSimulator"),
            kind: FileKind::Directory,
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
        let rules = HeuristicAnalyzer::new(Path::new("/Users/alice"));
        let en = rules.analyze(&record, Locale::EnUs);
        let pt = rules.analyze(&record, Locale::PtBr);
        assert_eq!(en.summary, "Xcode simulators and runtimes (CoreSimulator).");
        assert_eq!(
            pt.summary,
            "Simuladores e runtimes do Xcode (CoreSimulator)."
        );
        assert_eq!(en.level, pt.level);
        assert_eq!(en.commands[0].command, pt.commands[0].command);
        assert!(en.commands[0].description.starts_with("Removes"));
    }

    #[test]
    fn unknown_items_point_to_ai_or_manual_review() {
        let v = analyze("/Users/alice/stuff.bin", FileKind::File);
        assert_eq!(v.level, RiskLevel::Unknown);
        assert_eq!(v.recommendation, Recommendation::ReviewManually);
        assert!(v.summary.contains("stuff.bin"));
    }
}
