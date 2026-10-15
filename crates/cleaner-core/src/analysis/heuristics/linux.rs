//! Local rules for Linux (Debian and derivatives). Paths are matched lower-case with the home
//! folder as `~`.

use super::{Rule, cmd, steps, v};
use crate::analysis::verdict::Verdict;
use crate::domain::file_entry::FileKind;
use crate::domain::risk::{Recommendation, RiskCategory, RiskLevel};

pub(super) static RULES: &[Rule] = &[
    Rule {
        matches: |i| i.under("~/.local/share/trash"),
        verdict: |_, l| Verdict {
            commands: vec![cmd(
                l,
                "gio trash --empty",
                ("Empties the Trash", "Esvazia a Lixeira"),
            )],
            ..v(
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
        matches: |i| i.under("/var/log/journal"),
        verdict: |_, l| Verdict {
            commands: vec![
                cmd(
                    l,
                    "journalctl --disk-usage",
                    (
                        "Shows how much space the journal uses",
                        "Mostra quanto espaço o journal ocupa",
                    ),
                ),
                cmd(
                    l,
                    "sudo journalctl --vacuum-size=200M",
                    (
                        "Keeps only the most recent 200 MB of logs",
                        "Mantém só os 200 MB mais recentes de logs",
                    ),
                ),
            ],
            ..v(
                l,
                RiskLevel::Safe,
                RiskCategory::Log,
                (
                    "systemd journal (system logs).",
                    "Journal do systemd (logs do sistema).",
                ),
                (
                    "Old logs are only useful for diagnosing past problems. Shrink them with journalctl instead of deleting files.",
                    "Logs antigos só servem para diagnosticar problemas passados. Reduza com journalctl em vez de apagar arquivos.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("/var/cache/apt"),
        verdict: |_, l| Verdict {
            commands: vec![
                cmd(
                    l,
                    "sudo apt clean",
                    (
                        "Deletes downloaded package files",
                        "Apaga os pacotes baixados",
                    ),
                ),
                cmd(
                    l,
                    "sudo apt autoremove",
                    (
                        "Removes packages no longer needed",
                        "Remove pacotes que não são mais necessários",
                    ),
                ),
            ],
            ..v(
                l,
                RiskLevel::Safe,
                RiskCategory::Cache,
                ("APT package cache.", "Cache de pacotes do APT."),
                (
                    "Packages already installed; apt downloads them again if needed.",
                    "Pacotes já instalados; o apt baixa de novo se precisar.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("/var/lib/snapd"),
        verdict: |_, l| Verdict {
            commands: vec![
                cmd(
                    l,
                    "snap list --all",
                    (
                        "Lists installed snaps, including old disabled revisions",
                        "Lista os snaps instalados, incluindo revisões antigas desativadas",
                    ),
                ),
                cmd(
                    l,
                    "sudo snap set system refresh.retain=2",
                    (
                        "Keeps only two revisions of each snap from now on",
                        "Mantém só duas revisões de cada snap daqui em diante",
                    ),
                ),
            ],
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::AppBundle,
                ("Snap packages.", "Pacotes Snap."),
                (
                    "Installed snaps and older revisions. Remove apps with snap remove, never by deleting files.",
                    "Snaps instalados e revisões antigas. Remova apps com snap remove, nunca apagando arquivos.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("/var/lib/flatpak") || i.under("~/.local/share/flatpak"),
        verdict: |_, l| Verdict {
            commands: vec![cmd(
                l,
                "flatpak uninstall --unused",
                (
                    "Removes runtimes no app uses anymore",
                    "Remove runtimes que nenhum app usa mais",
                ),
            )],
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::AppBundle,
                ("Flatpak apps and runtimes.", "Apps e runtimes Flatpak."),
                (
                    "Remove apps with flatpak uninstall; unused runtimes can be cleaned up safely.",
                    "Remova apps com flatpak uninstall; runtimes sem uso podem ser limpos com segurança.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("/var/lib/docker"),
        verdict: |_, l| Verdict {
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
                ("Docker data.", "Dados do Docker."),
                (
                    "Images, containers and volumes. Deleting files here corrupts Docker; clean up with its commands.",
                    "Imagens, containers e volumes. Apagar arquivos aqui corrompe o Docker; limpe com os comandos dele.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("/var/lib/systemd/coredump"),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::Log,
                ("Crash dumps.", "Dumps de falhas."),
                (
                    "Memory dumps of programs that crashed; only useful for debugging.",
                    "Cópias de memória de programas que travaram; só servem para depuração.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| i.has_component("steamapps"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Steam › Library › right-click the game › Manage › Uninstall",
                    "Steam › Biblioteca › clique com o botão direito no jogo › Gerenciar › Desinstalar",
                )],
            ),
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::Game,
                ("Steam game files.", "Arquivos de jogo da Steam."),
                (
                    "Uninstall the game through Steam; saves usually stay in the cloud.",
                    "Desinstale o jogo pela Steam; os saves costumam ficar na nuvem.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| {
            i.kind != FileKind::File
                && i.path
                    .strip_prefix("/opt/")
                    .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
        },
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Remove it with the package manager (apt) or the vendor's uninstaller",
                    "Remova pelo gerenciador de pacotes (apt) ou pelo desinstalador do fabricante",
                )],
            ),
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::AppBundle,
                ("Installed application.", "Aplicativo instalado."),
                (
                    "Deleting the folder can leave the package manager inconsistent. Uninstall it properly.",
                    "Apagar a pasta pode deixar o gerenciador de pacotes inconsistente. Desinstale do jeito certo.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/.cache"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Close the related app before deleting",
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
        matches: |i| i.under("~/downloads") && i.ext_in(&["deb", "rpm", "appimage", "run"]),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::Installer,
                (
                    "Downloaded package or installer.",
                    "Pacote ou instalador baixado.",
                ),
                (
                    "Once the app is installed, the package can usually be downloaded again.",
                    "Depois que o app foi instalado, o pacote geralmente pode ser baixado de novo.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
];
