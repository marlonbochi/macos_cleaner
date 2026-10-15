//! Local rules for Windows. Paths are matched lower-case, with `/` separators and the home folder
//! as `~` (see `normalized_path`).

use super::{Rule, cmd, steps, v};
use crate::analysis::verdict::Verdict;
use crate::domain::file_entry::FileKind;
use crate::domain::risk::{Recommendation, RiskCategory, RiskLevel};

/// `X:/...`: any drive letter, since Windows may live on another drive.
fn on_any_drive(path: &str, rest: &str) -> bool {
    path.get(1..).is_some_and(|p| p == format!(":/{rest}") || p.starts_with(&format!(":/{rest}/")))
}

pub(super) static RULES: &[Rule] = &[
    Rule {
        matches: |i| i.under("~/appdata/local/temp") || on_any_drive(&i.path, "windows/temp"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Settings › System › Storage › Temporary files",
                    "Configurações › Sistema › Armazenamento › Arquivos temporários",
                )],
            ),
            ..v(
                l,
                RiskLevel::Safe,
                RiskCategory::Cache,
                ("Windows temporary files.", "Arquivos temporários do Windows."),
                (
                    "Programs leave temporary files here; files still in use are skipped when cleaning.",
                    "Programas deixam arquivos temporários aqui; arquivos em uso são ignorados na limpeza.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| on_any_drive(&i.path, "windows.old"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Settings › System › Storage › Temporary files › Previous Windows installation(s)",
                    "Configurações › Sistema › Armazenamento › Arquivos temporários › Instalações anteriores do Windows",
                )],
            ),
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::Backup,
                (
                    "Previous Windows installation.",
                    "Instalação anterior do Windows.",
                ),
                (
                    "Kept after an upgrade so you can roll back. Once you are happy with the current version, remove it through Windows' own cleanup.",
                    "Mantida após uma atualização para permitir voltar atrás. Se a versão atual está ok, remova pela limpeza do próprio Windows.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| on_any_drive(&i.path, "hiberfil.sys"),
        verdict: |_, l| Verdict {
            commands: vec![cmd(
                l,
                "powercfg /hibernate off",
                (
                    "Turns hibernation off and removes the file (run as administrator)",
                    "Desliga a hibernação e remove o arquivo (execute como administrador)",
                ),
            )],
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::System,
                ("Hibernation file.", "Arquivo de hibernação."),
                (
                    "Windows stores memory here to hibernate. It cannot be deleted directly; turning hibernation off removes it.",
                    "O Windows guarda a memória aqui para hibernar. Não pode ser apagado direto; desligar a hibernação o remove.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| {
            on_any_drive(&i.path, "pagefile.sys") || on_any_drive(&i.path, "swapfile.sys")
        },
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Dangerous,
                RiskCategory::System,
                ("Windows paging file.", "Arquivo de paginação do Windows."),
                (
                    "Virtual memory used by Windows. Keep it; its size is managed in System › About › Advanced system settings.",
                    "Memória virtual usada pelo Windows. Mantenha; o tamanho é gerenciado em Sistema › Sobre › Configurações avançadas do sistema.",
                ),
                Recommendation::Keep,
            )
        },
    },
    Rule {
        matches: |i| on_any_drive(&i.path, "windows/winsxs"),
        verdict: |_, l| Verdict {
            commands: vec![cmd(
                l,
                "Dism.exe /Online /Cleanup-Image /StartComponentCleanup",
                (
                    "Removes superseded Windows components (run as administrator)",
                    "Remove componentes substituídos do Windows (execute como administrador)",
                ),
            )],
            ..v(
                l,
                RiskLevel::Dangerous,
                RiskCategory::System,
                ("Windows component store.", "Repositório de componentes do Windows."),
                (
                    "Essential to Windows; never delete it by hand. DISM can clean up old versions of components safely.",
                    "Essencial para o Windows; nunca apague à mão. O DISM limpa versões antigas de componentes com segurança.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| on_any_drive(&i.path, "windows/softwaredistribution/download"),
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Settings › System › Storage › Temporary files › Windows Update Cleanup",
                    "Configurações › Sistema › Armazenamento › Arquivos temporários › Limpeza do Windows Update",
                )],
            ),
            ..v(
                l,
                RiskLevel::Safe,
                RiskCategory::Cache,
                ("Windows Update downloads.", "Downloads do Windows Update."),
                (
                    "Update packages already installed or downloaded again when needed.",
                    "Pacotes de atualização já instalados ou baixados de novo quando necessário.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.has_component("$recycle.bin"),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::Other,
                ("Items in the Recycle Bin.", "Itens na Lixeira."),
                (
                    "They were already discarded. Emptying the Recycle Bin frees the space.",
                    "Já foram descartados. Esvaziar a Lixeira libera o espaço.",
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
            let path = i.path.as_str();
            i.kind != FileKind::File
                && ["program files/", "program files (x86)/"]
                    .iter()
                    .any(|dir| {
                        path.get(3..)
                            .and_then(|p| p.strip_prefix(dir))
                            .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
                    })
        },
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Settings › Apps › Installed apps › … › Uninstall",
                    "Configurações › Aplicativos › Aplicativos instalados › … › Desinstalar",
                )],
            ),
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::AppBundle,
                ("Installed program.", "Programa instalado."),
                (
                    "Deleting the folder breaks the program and leaves registry entries behind. Uninstall it from Settings.",
                    "Apagar a pasta quebra o programa e deixa entradas no registro. Desinstale pelas Configurações.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| {
            i.under("~/appdata/local/npm-cache")
                || i.under("~/appdata/local/pip/cache")
                || i.under("~/.nuget/packages")
                || i.under("~/appdata/local/yarn/cache")
        },
        verdict: |_, l| Verdict {
            commands: vec![
                cmd(
                    l,
                    "npm cache clean --force",
                    ("Empties the npm cache", "Esvazia o cache do npm"),
                ),
                cmd(
                    l,
                    "pip cache purge",
                    ("Empties the pip cache", "Esvazia o cache do pip"),
                ),
                cmd(
                    l,
                    "dotnet nuget locals all --clear",
                    ("Empties the NuGet caches", "Esvazia os caches do NuGet"),
                ),
            ],
            ..v(
                l,
                RiskLevel::Safe,
                RiskCategory::Cache,
                ("Developer package cache.", "Cache de pacotes de desenvolvimento."),
                (
                    "Packages are downloaded again when a project needs them.",
                    "Os pacotes são baixados de novo quando um projeto precisar.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.path.ends_with("ext4.vhdx") || i.path.ends_with("docker_data.vhdx"),
        verdict: |_, l| Verdict {
            commands: vec![
                cmd(
                    l,
                    "docker system prune -a",
                    (
                        "Removes unused Docker images, containers and networks",
                        "Remove imagens, containers e redes não usados do Docker",
                    ),
                ),
                cmd(
                    l,
                    "wsl --shutdown",
                    (
                        "Stops WSL so the disk can be compacted",
                        "Para o WSL para que o disco possa ser compactado",
                    ),
                ),
            ],
            ..v(
                l,
                RiskLevel::Caution,
                RiskCategory::VmImage,
                ("WSL or Docker virtual disk.", "Disco virtual do WSL ou do Docker."),
                (
                    "It holds a whole Linux system or Docker data. Clean up inside it; deleting the file loses everything in it.",
                    "Contém um sistema Linux inteiro ou dados do Docker. Limpe por dentro; apagar o arquivo perde tudo.",
                ),
                Recommendation::UseAppOrCommand,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/onedrive"),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Caution,
                RiskCategory::UserDocument,
                ("Folder synced with OneDrive.", "Pasta sincronizada com o OneDrive."),
                (
                    "Deleting here also deletes it in the cloud. To free space, right-click it and choose \"Free up space\" to keep it only online.",
                    "Apagar aqui apaga também na nuvem. Para liberar espaço, clique com o botão direito e escolha \"Liberar espaço\" para mantê-la só online.",
                ),
                Recommendation::MoveToCloudOrExternal,
            )
        },
    },
    Rule {
        matches: |i| i.under("~/downloads") && i.ext_in(&["exe", "msi", "msix", "appx"]),
        verdict: |_, l| {
            v(
                l,
                RiskLevel::Safe,
                RiskCategory::Installer,
                ("Downloaded installer.", "Instalador baixado."),
                (
                    "Once the program is installed, the installer can usually be downloaded again.",
                    "Depois que o programa foi instalado, o instalador geralmente pode ser baixado de novo.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
    Rule {
        matches: |i| {
            i.under("~/appdata/local")
                && (i.has_component("cache")
                    || i.has_component("cache2")
                    || i.has_component("code cache")
                    || i.has_component("gpucache")
                    || i.has_component("inetcache"))
        },
        verdict: |_, l| Verdict {
            steps: steps(
                l,
                &[(
                    "Close the related program before deleting",
                    "Feche o programa correspondente antes de apagar",
                )],
            ),
            ..v(
                l,
                RiskLevel::Safe,
                RiskCategory::Cache,
                ("Program or browser cache.", "Cache de programa ou navegador."),
                (
                    "Caches are recreated automatically; the program may be slower the first time it opens.",
                    "Caches são recriados automaticamente; o programa pode ficar mais lento na primeira abertura.",
                ),
                Recommendation::DeleteSafely,
            )
        },
    },
];
