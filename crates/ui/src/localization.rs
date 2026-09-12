use gpui::{App, Global, SharedString};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum UiLanguage {
    #[default]
    English,
    French,
    German,
    Spanish,
}

impl Global for UiLanguage {}

pub fn set_language(language: UiLanguage, cx: &mut App) {
    if cx.try_global::<UiLanguage>().copied().unwrap_or_default() != language {
        cx.set_global(language);
        cx.refresh_windows();
    }
}

pub fn localized(text: &'static str, cx: &App) -> SharedString {
    let language = cx.try_global::<UiLanguage>().copied().unwrap_or_default();
    let translations = match text {
        "Profile" => ["Profil", "Profil", "Perfil"],
        "General" => ["Général", "Allgemein", "General"],
        "Appearance" => ["Apparence", "Darstellung", "Apariencia"],
        "Keymap" => ["Raccourcis clavier", "Tastenbelegung", "Atajos de teclado"],
        "Editor" => ["Éditeur", "Editor", "Editor"],
        "Languages & Tools" => [
            "Langages et outils",
            "Sprachen und Werkzeuge",
            "Lenguajes y herramientas",
        ],
        "Search & Files" => [
            "Recherche et fichiers",
            "Suche und Dateien",
            "Búsqueda y archivos",
        ],
        "Window & Layout" => [
            "Fenêtre et disposition",
            "Fenster und Layout",
            "Ventana y diseño",
        ],
        "Panels" => ["Panneaux", "Bereiche", "Paneles"],
        "Debugger" => ["Débogueur", "Debugger", "Depurador"],
        "Terminal" => ["Terminal", "Terminal", "Terminal"],
        "Version Control" => [
            "Gestion de versions",
            "Versionsverwaltung",
            "Control de versiones",
        ],
        "Collaboration" => ["Collaboration", "Zusammenarbeit", "Colaboración"],
        "AI" => ["IA", "KI", "IA"],
        "Network" => ["Réseau", "Netzwerk", "Red"],
        "Developer" => ["Développeur", "Entwickler", "Desarrollador"],
        "Back to app" => [
            "Retour à l’application",
            "Zurück zur App",
            "Volver a la aplicación",
        ],
        "Search settings…" => [
            "Rechercher un paramètre…",
            "Einstellungen suchen…",
            "Buscar ajustes…",
        ],
        "Themes…" => ["Thèmes…", "Designs…", "Temas…"],
        "Icon Themes…" => ["Thèmes d’icônes…", "Symboldesigns…", "Temas de iconos…"],
        "Extensions" => ["Extensions", "Erweiterungen", "Extensiones"],
        "Permissions" => ["Autorisations", "Berechtigungen", "Permisos"],
        "Default permissions" => [
            "Autorisations par défaut",
            "Standardberechtigungen",
            "Permisos predeterminados",
        ],
        "Full access" => ["Accès complet", "Vollzugriff", "Acceso completo"],
        "Projectless task folder" => [
            "Dossier des tâches sans projet",
            "Ordner für Aufgaben ohne Projekt",
            "Carpeta de tareas sin proyecto",
        ],
        "Default file open destination" => [
            "Application d’ouverture des fichiers",
            "Standardziel zum Öffnen von Dateien",
            "Destino predeterminado para abrir archivos",
        ],
        "Language" => ["Langue", "Sprache", "Idioma"],
        "Show in menu bar" => [
            "Afficher dans la barre des menus",
            "In der Menüleiste anzeigen",
            "Mostrar en la barra de menús",
        ],
        "Bottom panel" => ["Panneau inférieur", "Unterer Bereich", "Panel inferior"],
        "Default terminal location" => [
            "Emplacement du terminal",
            "Standardposition des Terminals",
            "Ubicación predeterminada del terminal",
        ],
        "Prevent sleep while running" => [
            "Empêcher la veille pendant l’exécution",
            "Ruhezustand während der Ausführung verhindern",
            "Evitar la suspensión durante la ejecución",
        ],
        "Speed" => ["Vitesse", "Geschwindigkeit", "Velocidad"],
        "Standard" => ["Standard", "Standard", "Estándar"],
        "Fast" => ["Rapide", "Schnell", "Rápido"],
        "Change" => ["Modifier", "Ändern", "Cambiar"],
        "Bottom" => ["En bas", "Unten", "Abajo"],
        "Right" => ["À droite", "Rechts", "Derecha"],
        "System default" => [
            "Application par défaut du système",
            "Systemstandard",
            "Predeterminado del sistema",
        ],
        "New thread" => [
            "Nouvelle conversation",
            "Neue Unterhaltung",
            "Nueva conversación",
        ],
        "Switch project" => [
            "Changer de projet",
            "Projekt wechseln",
            "Cambiar de proyecto",
        ],
        "Search threads…" => [
            "Rechercher une conversation…",
            "Unterhaltungen suchen…",
            "Buscar conversaciones…",
        ],
        "Search files…" => [
            "Rechercher des fichiers…",
            "Dateien suchen…",
            "Buscar archivos…",
        ],
        "Account & settings" => [
            "Compte et paramètres",
            "Konto und Einstellungen",
            "Cuenta y ajustes",
        ],
        "Today" => ["Aujourd’hui", "Heute", "Hoy"],
        "Previous 7 days" => ["7 derniers jours", "Letzte 7 Tage", "Últimos 7 días"],
        "What would you like to build?" => [
            "Que souhaitez-vous créer ?",
            "Was möchtest du entwickeln?",
            "¿Qué te gustaría crear?",
        ],
        "Ask anything…" => [
            "Posez votre question…",
            "Stelle eine Frage…",
            "Pregunta lo que quieras…",
        ],
        "Local" => ["Local", "Lokal", "Local"],
        "Default" => ["Par défaut", "Standard", "Predeterminado"],
        "Retry" => ["Réessayer", "Erneut versuchen", "Reintentar"],
        "Stop" => ["Arrêter", "Anhalten", "Detener"],
        "Continue" => ["Continuer", "Fortsetzen", "Continuar"],
        "Reconnect" => ["Reconnecter", "Erneut verbinden", "Reconectar"],
        "Review changes" => [
            "Examiner les modifications",
            "Änderungen prüfen",
            "Revisar cambios",
        ],
        "Choose another model" => [
            "Choisir un autre modèle",
            "Anderes Modell wählen",
            "Elegir otro modelo",
        ],
        "Models" => ["Modèles", "Modelle", "Modelos"],
        "Connections" => ["Connexions", "Verbindungen", "Conexiones"],
        "Default model" => [
            "Modèle par défaut",
            "Standardmodell",
            "Modelo predeterminado",
        ],
        "Reasoning" => ["Raisonnement", "Denkaufwand", "Razonamiento"],
        "Context size" => ["Taille du contexte", "Kontextgröße", "Tamaño del contexto"],
        "Add connection" => [
            "Ajouter une connexion",
            "Verbindung hinzufügen",
            "Añadir conexión",
        ],
        "Test connection" => [
            "Tester la connexion",
            "Verbindung testen",
            "Probar conexión",
        ],
        "Working directory" => [
            "Répertoire de travail",
            "Arbeitsverzeichnis",
            "Directorio de trabajo",
        ],
        "Name" => ["Nom", "Name", "Nombre"],
        "Command" => ["Commande", "Befehl", "Comando"],
        "Arguments" => ["Arguments", "Argumente", "Argumentos"],
        "Environment variables" => [
            "Variables d’environnement",
            "Umgebungsvariablen",
            "Variables de entorno",
        ],
        "Save" => ["Enregistrer", "Speichern", "Guardar"],
        "Cancel" => ["Annuler", "Abbrechen", "Cancelar"],
        "Connect" => ["Connecter", "Verbinden", "Conectar"],
        "Configure" => ["Configurer", "Konfigurieren", "Configurar"],
        "Connected" => ["Connecté", "Verbunden", "Conectado"],
        "Not connected" => ["Non connecté", "Nicht verbunden", "Sin conexión"],
        "Lifetime tokens" => ["Total de jetons", "Token insgesamt", "Tokens totales"],
        "Peak tokens" => ["Pic de jetons", "Token-Höchstwert", "Máximo de tokens"],
        "Longest chat" => [
            "Conversation la plus longue",
            "Längste Unterhaltung",
            "Conversación más larga",
        ],
        "Current streak" => ["Série actuelle", "Aktuelle Serie", "Racha actual"],
        "Longest streak" => ["Plus longue série", "Längste Serie", "Racha más larga"],
        "Token activity" => [
            "Activité des jetons",
            "Token-Aktivität",
            "Actividad de tokens",
        ],
        "Daily" => ["Par jour", "Täglich", "Diaria"],
        "Weekly" => ["Par semaine", "Wöchentlich", "Semanal"],
        "Cumulative" => ["Cumulée", "Kumulativ", "Acumulada"],
        "Less" => ["Moins", "Weniger", "Menos"],
        "More" => ["Plus", "Mehr", "Más"],
        "Run" => ["Exécuter", "Ausführen", "Ejecutar"],
        "Debug" => ["Déboguer", "Debuggen", "Depurar"],
        "Build" => ["Compiler", "Erstellen", "Compilar"],
        "Test" => ["Tester", "Testen", "Probar"],
        "New File" => ["Nouveau fichier", "Neue Datei", "Nuevo archivo"],
        "New Folder" => ["Nouveau dossier", "Neuer Ordner", "Nueva carpeta"],
        "Open Folder" => ["Ouvrir un dossier", "Ordner öffnen", "Abrir carpeta"],
        "Search projects…" => [
            "Rechercher des projets…",
            "Projekte suchen…",
            "Buscar proyectos…",
        ],
        "THIS WINDOW" => ["CETTE FENÊTRE", "DIESES FENSTER", "ESTA VENTANA"],
        "RECENT PROJECTS" => ["PROJETS RÉCENTS", "LETZTE PROJEKTE", "PROYECTOS RECIENTES"],
        "ADD PROJECT" => ["AJOUTER UN PROJET", "PROJEKT HINZUFÜGEN", "AÑADIR PROYECTO"],
        "Local folder" => ["Dossier local", "Lokaler Ordner", "Carpeta local"],
        "Open a folder or existing Git checkout" => [
            "Ouvrir un dossier ou un dépôt Git local",
            "Ordner oder vorhandenes Git-Projekt öffnen",
            "Abrir una carpeta o un repositorio Git local",
        ],
        "Git URL" => ["URL Git", "Git-URL", "URL de Git"],
        "Clone a repository from a remote URL" => [
            "Cloner un dépôt depuis une URL distante",
            "Repository über eine Remote-URL klonen",
            "Clonar un repositorio desde una URL remota",
        ],
        "GitHub repository" => ["Dépôt GitHub", "GitHub-Repository", "Repositorio de GitHub"],
        "Find and clone a GitHub repository" => [
            "Rechercher et cloner un dépôt GitHub",
            "GitHub-Repository suchen und klonen",
            "Buscar y clonar un repositorio de GitHub",
        ],
        "Open Remote Folder" => [
            "Ouvrir un dossier distant",
            "Remote-Ordner öffnen",
            "Abrir carpeta remota",
        ],
        "Choose where file links from conversations open." => [
            "Choisissez où ouvrir les liens vers des fichiers dans les conversations.",
            "Wähle, wo Dateilinks aus Unterhaltungen geöffnet werden.",
            "Elige dónde abrir los enlaces a archivos de las conversaciones.",
        ],
        "Default location for tasks started outside a project." => [
            "Emplacement par défaut des tâches sans projet.",
            "Standardordner für Aufgaben außerhalb eines Projekts.",
            "Ubicación predeterminada de las tareas sin proyecto.",
        ],
        "Choose the language used in the app." => [
            "Choisissez la langue de l’application.",
            "Wähle die Sprache der App.",
            "Elige el idioma de la aplicación.",
        ],
        "Keep Zloppenheimer in the menu bar when the window is closed." => [
            "Garder Zloppenheimer dans la barre des menus lorsque la fenêtre est fermée.",
            "Zloppenheimer nach dem Schließen des Fensters in der Menüleiste behalten.",
            "Mantener Zloppenheimer en la barra de menús al cerrar la ventana.",
        ],
        "Show the bottom panel control in the app header." => [
            "Afficher le bouton du panneau inférieur dans l’en-tête.",
            "Schaltfläche für den unteren Bereich in der Kopfzeile anzeigen.",
            "Mostrar el control del panel inferior en la cabecera.",
        ],
        "Where terminal shortcuts open new terminal tabs." => [
            "Emplacement des nouveaux onglets de terminal.",
            "Ziel für neue Terminal-Tabs über Tastenkürzel.",
            "Dónde abren los atajos las nuevas pestañas de terminal.",
        ],
        "Keep your computer awake while an agent runs a task." => [
            "Empêcher la mise en veille pendant une tâche de l’agent.",
            "Computer während einer Agent-Aufgabe wach halten.",
            "Mantener el equipo activo mientras un agente ejecuta una tarea.",
        ],
        "Use faster responses when supported by the selected model." => [
            "Utiliser les réponses rapides si le modèle sélectionné le permet.",
            "Schnellere Antworten verwenden, wenn das gewählte Modell sie unterstützt.",
            "Usar respuestas más rápidas si el modelo seleccionado lo admite.",
        ],
        "Agents can read and edit workspace files, and ask for additional access when needed." => [
            "Les agents peuvent lire et modifier les fichiers du projet et demander un accès supplémentaire si nécessaire.",
            "Agenten können Dateien im Arbeitsbereich lesen und bearbeiten sowie bei Bedarf weiteren Zugriff anfragen.",
            "Los agentes pueden leer y editar archivos del proyecto y solicitar acceso adicional cuando sea necesario.",
        ],
        "Allow agents to edit files outside the workspace and run network commands without approval. This increases the risk of data loss or unexpected changes." => {
            [
                "Autoriser les agents à modifier des fichiers hors du projet et à exécuter des commandes réseau sans approbation. Cela augmente le risque de perte de données ou de modifications inattendues.",
                "Agenten dürfen Dateien außerhalb des Arbeitsbereichs bearbeiten und Netzwerkbefehle ohne Genehmigung ausführen. Dies erhöht das Risiko von Datenverlust oder unerwarteten Änderungen.",
                "Permitir que los agentes editen archivos fuera del proyecto y ejecuten comandos de red sin aprobación. Esto aumenta el riesgo de pérdida de datos o cambios inesperados.",
            ]
        }
        "In app" => ["Dans l’application", "In der App", "En la aplicación"],
        "Default application" => [
            "Application par défaut",
            "Standardanwendung",
            "Aplicación predeterminada",
        ],
        "Choose the models in your chat picker and set your defaults." => [
            "Choisissez les modèles du sélecteur et vos valeurs par défaut.",
            "Wähle die Modelle für deine Unterhaltungen und lege die Standardwerte fest.",
            "Elige los modelos del selector y establece tus valores predeterminados.",
        ],
        "Defaults" => [
            "Valeurs par défaut",
            "Standardwerte",
            "Valores predeterminados",
        ],
        "Context window" => [
            "Fenêtre de contexte",
            "Kontextfenster",
            "Ventana de contexto",
        ],
        "Used when you choose Default in the chat composer." => [
            "Utilisé lorsque vous choisissez Par défaut dans une conversation.",
            "Wird bei der Auswahl „Standard“ im Nachrichteneditor verwendet.",
            "Se usa al elegir Predeterminado en el editor del chat.",
        ],
        "Starting effort for new conversations." => [
            "Niveau initial pour les nouvelles conversations.",
            "Anfänglicher Denkaufwand für neue Unterhaltungen.",
            "Esfuerzo inicial para nuevas conversaciones.",
        ],
        "Starting context size when supported by the model." => [
            "Taille initiale du contexte si le modèle le permet.",
            "Anfängliche Kontextgröße, sofern vom Modell unterstützt.",
            "Tamaño inicial del contexto si el modelo lo admite.",
        ],
        "Model default" => ["Valeur du modèle", "Modellstandard", "Valor del modelo"],
        "Models in your picker" => [
            "Modèles du sélecteur",
            "Modelle in deiner Auswahl",
            "Modelos del selector",
        ],
        "Search available models…" => [
            "Rechercher un modèle…",
            "Verfügbare Modelle suchen…",
            "Buscar modelos disponibles…",
        ],
        "Show in model picker" => [
            "Afficher dans le sélecteur",
            "In Modellauswahl anzeigen",
            "Mostrar en el selector de modelos",
        ],
        "Hidden from model picker" => [
            "Masqué dans le sélecteur",
            "In Modellauswahl ausgeblendet",
            "Oculto en el selector de modelos",
        ],
        "Model connections" => [
            "Connexions aux modèles",
            "Modellverbindungen",
            "Conexiones de modelos",
        ],
        "Connect a provider to make its models available." => [
            "Connectez un fournisseur pour accéder à ses modèles.",
            "Verbinde einen Anbieter, um dessen Modelle verfügbar zu machen.",
            "Conecta un proveedor para acceder a sus modelos.",
        ],
        "＋ Connect model" => [
            "＋ Connecter un modèle",
            "＋ Modell verbinden",
            "＋ Conectar modelo",
        ],
        "Advanced AI settings" => [
            "Paramètres avancés de l’IA",
            "Erweiterte KI-Einstellungen",
            "Ajustes avanzados de IA",
        ],
        "← AI settings" => [
            "← Paramètres de l’IA",
            "← KI-Einstellungen",
            "← Ajustes de IA",
        ],
        "Account connected" => ["Compte connecté", "Konto verbunden", "Cuenta conectada"],
        "ACP agent installed" => [
            "Agent ACP installé",
            "ACP-Agent installiert",
            "Agente ACP instalado",
        ],
        "Disabled" => ["Désactivé", "Deaktiviert", "Desactivado"],
        "Installed" => ["Installé", "Installiert", "Instalado"],
        "Use + to add a provider or custom ACP agent." => [
            "Utilisez + pour ajouter un fournisseur ou un agent ACP personnalisé.",
            "Mit + einen Anbieter oder eigenen ACP-Agenten hinzufügen.",
            "Usa + para añadir un proveedor o agente ACP personalizado.",
        ],
        "Display name" => ["Nom affiché", "Anzeigename", "Nombre visible"],
        "Name shown in your connections." => [
            "Nom affiché dans vos connexions.",
            "Name, der in den Verbindungen angezeigt wird.",
            "Nombre que aparece en tus conexiones.",
        ],
        "Runtime" => ["Exécution", "Laufzeit", "Ejecución"],
        "Binary path" => ["Chemin du programme", "Programmpfad", "Ruta del ejecutable"],
        "Executable used for this connection." => [
            "Programme utilisé pour cette connexion.",
            "Für diese Verbindung verwendetes Programm.",
            "Ejecutable utilizado para esta conexión.",
        ],
        "Launch arguments" => [
            "Arguments de lancement",
            "Startargumente",
            "Argumentos de inicio",
        ],
        "Additional arguments passed on startup." => [
            "Arguments supplémentaires au démarrage.",
            "Zusätzliche Argumente beim Start.",
            "Argumentos adicionales al iniciar.",
        ],
        "Environment" => ["Environnement", "Umgebung", "Entorno"],
        "Variables" => ["Variables", "Variablen", "Variables"],
        "Keys, base URLs, and connection-specific settings." => [
            "Clés, URL et paramètres propres à la connexion.",
            "Schlüssel, Basis-URLs und Verbindungseinstellungen.",
            "Claves, URL base y ajustes de la conexión.",
        ],
        "Configure connection" => [
            "Configurer la connexion",
            "Verbindung konfigurieren",
            "Configurar conexión",
        ],
        "Remove connection" => [
            "Supprimer la connexion",
            "Verbindung entfernen",
            "Eliminar conexión",
        ],
        "ADD CONNECTION" => [
            "AJOUTER UNE CONNEXION",
            "VERBINDUNG HINZUFÜGEN",
            "AÑADIR CONEXIÓN",
        ],
        "Provider account" => ["Compte fournisseur", "Anbieterkonto", "Cuenta de proveedor"],
        "Choose a provider and sign in." => [
            "Choisissez un fournisseur et connectez-vous.",
            "Wähle einen Anbieter und melde dich an.",
            "Elige un proveedor e inicia sesión.",
        ],
        "API key" => ["Clé API", "API-Schlüssel", "Clave API"],
        "Connect a model provider with your own key." => [
            "Connectez un fournisseur avec votre propre clé.",
            "Verbinde einen Modellanbieter mit deinem eigenen Schlüssel.",
            "Conecta un proveedor de modelos con tu propia clave.",
        ],
        "Custom ACP agent" => [
            "Agent ACP personnalisé",
            "Eigener ACP-Agent",
            "Agente ACP personalizado",
        ],
        "Run an agent using an ACP command." => [
            "Exécuter un agent avec une commande ACP.",
            "Einen Agenten über einen ACP-Befehl ausführen.",
            "Ejecutar un agente mediante un comando ACP.",
        ],
        "Add custom ACP agent" => [
            "Ajouter un agent ACP personnalisé",
            "Eigenen ACP-Agenten hinzufügen",
            "Añadir agente ACP personalizado",
        ],
        "Configure ACP agent" => [
            "Configurer l’agent ACP",
            "ACP-Agent konfigurieren",
            "Configurar agente ACP",
        ],
        "＋ Add variable" => [
            "＋ Ajouter une variable",
            "＋ Variable hinzufügen",
            "＋ Añadir variable",
        ],
        "Add agent" => ["Ajouter l’agent", "Agent hinzufügen", "Añadir agente"],
        "Connecting…" => ["Connexion…", "Verbindung wird hergestellt…", "Conectando…"],
        "Filter action names…" => [
            "Filtrer les actions…",
            "Aktionsnamen filtern…",
            "Filtrar nombres de acciones…",
        ],
        "Edit in JSON" => ["Modifier en JSON", "In JSON bearbeiten", "Editar en JSON"],
        "Create keybinding" => [
            "Créer un raccourci",
            "Tastenkürzel erstellen",
            "Crear atajo de teclado",
        ],
        "Action" => ["Action", "Aktion", "Acción"],
        "Keystrokes" => ["Raccourci", "Tastenfolge", "Teclas"],
        "Context" => ["Contexte", "Kontext", "Contexto"],
        "Source" => ["Source", "Quelle", "Origen"],
        "Select a binding to edit its shortcut" => [
            "Sélectionnez un raccourci pour le modifier",
            "Wähle eine Belegung, um das Tastenkürzel zu bearbeiten",
            "Selecciona un atajo para editarlo",
        ],
        "Search" => ["Rechercher", "Suchen", "Buscar"],
        "Searching…" => ["Recherche…", "Suche…", "Buscando…"],
        "Clone" => ["Cloner", "Klonen", "Clonar"],
        "Search public GitHub repositories…" => [
            "Rechercher des dépôts GitHub publics…",
            "Öffentliche GitHub-Repositories suchen…",
            "Buscar repositorios públicos de GitHub…",
        ],
        "Auto detect" => [
            "Détection automatique",
            "Automatisch erkennen",
            "Detectar automáticamente",
        ],
        _ => return text.into(),
    };
    match language {
        UiLanguage::English => text,
        UiLanguage::French => translations[0],
        UiLanguage::German => translations[1],
        UiLanguage::Spanish => translations[2],
    }
    .into()
}
