#include "mainwindow.h"
#include "graphview.h"

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QMenuBar>
#include <QMenu>
#include <QAction>
#include <QFileDialog>
#include <QMessageBox>
#include <QApplication>
#include <QSplitter>
#include <QGroupBox>
#include <QTextEdit>
#include <QDir>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , m_analysisProcess(nullptr)
{
    setWindowTitle("Mr. Hedgehog - Rust Call Graph Analyzer");
    setMinimumSize(1200, 800);
    resize(1400, 900);
    
    // Apply modern dark theme
    setStyleSheet(R"(
        QMainWindow {
            background-color: #1e1e2e;
        }
        QMenuBar {
            background-color: #181825;
            color: #cdd6f4;
            border-bottom: 1px solid #313244;
        }
        QMenuBar::item:selected {
            background-color: #45475a;
        }
        QMenu {
            background-color: #1e1e2e;
            color: #cdd6f4;
            border: 1px solid #313244;
        }
        QMenu::item:selected {
            background-color: #45475a;
        }
        QToolBar {
            background-color: #181825;
            border: none;
            spacing: 8px;
            padding: 4px;
        }
        QToolButton {
            background-color: transparent;
            color: #cdd6f4;
            border: none;
            padding: 8px 12px;
            border-radius: 6px;
        }
        QToolButton:hover {
            background-color: #313244;
        }
        QToolButton:pressed {
            background-color: #45475a;
        }
        QDockWidget {
            color: #cdd6f4;
            titlebar-close-icon: none;
        }
        QDockWidget::title {
            background-color: #181825;
            padding: 8px;
            border-bottom: 1px solid #313244;
        }
        QLineEdit {
            background-color: #313244;
            color: #cdd6f4;
            border: 1px solid #45475a;
            border-radius: 6px;
            padding: 8px 12px;
            selection-background-color: #89b4fa;
        }
        QLineEdit:focus {
            border-color: #89b4fa;
        }
        QPushButton {
            background-color: #89b4fa;
            color: #1e1e2e;
            border: none;
            border-radius: 6px;
            padding: 10px 20px;
            font-weight: bold;
        }
        QPushButton:hover {
            background-color: #b4befe;
        }
        QPushButton:pressed {
            background-color: #74c7ec;
        }
        QPushButton:disabled {
            background-color: #45475a;
            color: #6c7086;
        }
        QPushButton#clearBtn {
            background-color: #f38ba8;
        }
        QPushButton#clearBtn:hover {
            background-color: #eba0ac;
        }
        QListWidget {
            background-color: #1e1e2e;
            color: #cdd6f4;
            border: 1px solid #313244;
            border-radius: 6px;
        }
        QListWidget::item {
            padding: 8px;
            border-bottom: 1px solid #313244;
        }
        QListWidget::item:selected {
            background-color: #45475a;
        }
        QListWidget::item:hover {
            background-color: #313244;
        }
        QStatusBar {
            background-color: #181825;
            color: #a6adc8;
            border-top: 1px solid #313244;
        }
        QLabel {
            color: #cdd6f4;
        }
        QGroupBox {
            color: #cdd6f4;
            border: 1px solid #313244;
            border-radius: 8px;
            margin-top: 12px;
            padding-top: 12px;
        }
        QGroupBox::title {
            subcontrol-origin: margin;
            left: 12px;
            padding: 0 8px;
        }
    )");
    
    setupUI();
    loadSettings();
}

MainWindow::~MainWindow()
{
    saveSettings();
    if (m_analysisProcess) {
        m_analysisProcess->kill();
        delete m_analysisProcess;
    }
}

void MainWindow::setupUI()
{
    setupMenuBar();
    setupToolBar();
    setupSidebar();
    setupCentralWidget();
    setupStatusBar();
}

void MainWindow::setupMenuBar()
{
    QMenuBar *menuBar = this->menuBar();
    
    // File menu
    QMenu *fileMenu = menuBar->addMenu("&File");
    
    QAction *openAction = fileMenu->addAction("&Open Folder...");
    openAction->setShortcut(QKeySequence::Open);
    connect(openAction, &QAction::triggered, this, &MainWindow::selectFolder);
    
    fileMenu->addSeparator();
    
    QAction *exitAction = fileMenu->addAction("E&xit");
    exitAction->setShortcut(QKeySequence::Quit);
    connect(exitAction, &QAction::triggered, this, &QMainWindow::close);
    
    // Analysis menu
    QMenu *analysisMenu = menuBar->addMenu("&Analysis");
    
    QAction *runAction = analysisMenu->addAction("&Run Analysis");
    runAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_R));
    connect(runAction, &QAction::triggered, this, &MainWindow::runAnalysis);
    
    QAction *clearAction = analysisMenu->addAction("&Clear Results");
    connect(clearAction, &QAction::triggered, this, &MainWindow::clearResults);
    
    // Help menu
    QMenu *helpMenu = menuBar->addMenu("&Help");
    
    QAction *aboutAction = helpMenu->addAction("&About Mr. Hedgehog");
    connect(aboutAction, &QAction::triggered, this, &MainWindow::showAbout);
}

void MainWindow::setupToolBar()
{
    m_toolbar = addToolBar("Main Toolbar");
    m_toolbar->setMovable(false);
    m_toolbar->setIconSize(QSize(24, 24));
    
    m_toolbar->addAction("📂 Open", this, &MainWindow::selectFolder);
    m_toolbar->addAction("▶️ Analyze", this, &MainWindow::runAnalysis);
    m_toolbar->addAction("🗑️ Clear", this, &MainWindow::clearResults);
    m_toolbar->addSeparator();
}

void MainWindow::setupSidebar()
{
    m_sidebarDock = new QDockWidget("Project", this);
    m_sidebarDock->setFeatures(QDockWidget::NoDockWidgetFeatures);
    m_sidebarDock->setAllowedAreas(Qt::LeftDockWidgetArea);
    
    QWidget *sidebarWidget = new QWidget();
    QVBoxLayout *layout = new QVBoxLayout(sidebarWidget);
    layout->setContentsMargins(12, 12, 12, 12);
    layout->setSpacing(12);
    
    // Folder selection group
    QGroupBox *folderGroup = new QGroupBox("Workspace Folder");
    QVBoxLayout *folderLayout = new QVBoxLayout(folderGroup);
    
    QHBoxLayout *pathLayout = new QHBoxLayout();
    m_folderPath = new QLineEdit();
    m_folderPath->setPlaceholderText("Select a Rust project folder...");
    m_folderPath->setReadOnly(true);
    pathLayout->addWidget(m_folderPath);
    
    m_browseBtn = new QPushButton("Browse");
    m_browseBtn->setFixedWidth(80);
    connect(m_browseBtn, &QPushButton::clicked, this, &MainWindow::selectFolder);
    pathLayout->addWidget(m_browseBtn);
    
    folderLayout->addLayout(pathLayout);
    layout->addWidget(folderGroup);
    
    // Actions group
    QGroupBox *actionsGroup = new QGroupBox("Actions");
    QVBoxLayout *actionsLayout = new QVBoxLayout(actionsGroup);
    
    m_analyzeBtn = new QPushButton("🔍 Run Analysis");
    m_analyzeBtn->setEnabled(false);
    connect(m_analyzeBtn, &QPushButton::clicked, this, &MainWindow::runAnalysis);
    actionsLayout->addWidget(m_analyzeBtn);
    
    m_clearBtn = new QPushButton("Clear Results");
    m_clearBtn->setObjectName("clearBtn");
    connect(m_clearBtn, &QPushButton::clicked, this, &MainWindow::clearResults);
    actionsLayout->addWidget(m_clearBtn);
    
    layout->addWidget(actionsGroup);
    
    // File list
    QGroupBox *filesGroup = new QGroupBox("Source Files");
    QVBoxLayout *filesLayout = new QVBoxLayout(filesGroup);
    
    m_fileList = new QListWidget();
    m_fileList->setMinimumHeight(200);
    filesLayout->addWidget(m_fileList);
    
    layout->addWidget(filesGroup);
    layout->addStretch();
    
    m_sidebarDock->setWidget(sidebarWidget);
    m_sidebarDock->setMinimumWidth(300);
    addDockWidget(Qt::LeftDockWidgetArea, m_sidebarDock);
}

void MainWindow::setupCentralWidget()
{
    m_graphView = new GraphView(this);
    setCentralWidget(m_graphView);
}

void MainWindow::setupStatusBar()
{
    m_statusLabel = new QLabel("Ready - Select a folder to begin");
    statusBar()->addWidget(m_statusLabel);
}

void MainWindow::selectFolder()
{
    QString folder = QFileDialog::getExistingDirectory(
        this,
        "Select Rust Project Folder",
        QDir::homePath(),
        QFileDialog::ShowDirsOnly | QFileDialog::DontResolveSymlinks
    );
    
    if (!folder.isEmpty()) {
        m_currentFolder = folder;
        m_folderPath->setText(folder);
        updateAnalyzeButton();
        
        // List .rs files
        m_fileList->clear();
        QDir dir(folder);
        QStringList filters;
        filters << "*.rs";
        
        QFileInfoList files = dir.entryInfoList(filters, QDir::Files, QDir::Name);
        
        // Also check src/ subdirectory
        QDir srcDir(folder + "/src");
        if (srcDir.exists()) {
            files.append(srcDir.entryInfoList(filters, QDir::Files, QDir::Name));
        }
        
        for (const QFileInfo &file : files) {
            m_fileList->addItem(file.fileName());
        }
        
        m_statusLabel->setText(QString("Loaded: %1 (%2 .rs files)").arg(folder).arg(files.size()));
    }
}

void MainWindow::updateAnalyzeButton()
{
    m_analyzeBtn->setEnabled(!m_currentFolder.isEmpty());
}

void MainWindow::runAnalysis()
{
    if (m_currentFolder.isEmpty()) {
        QMessageBox::warning(this, "No Folder Selected", 
            "Please select a Rust project folder first.");
        return;
    }
    
    m_statusLabel->setText("Running analysis...");
    m_analyzeBtn->setEnabled(false);
    
    // Find backend executable
    QString backendPath = QApplication::applicationDirPath() + "/mr_hedgehog";
    if (!QFile::exists(backendPath)) {
        // Try relative path during development
        backendPath = QDir::currentPath() + "/target/release/mr_hedgehog";
    }
    
    if (!QFile::exists(backendPath)) {
        m_graphView->showPlaceholder("Backend not found.\nPlease ensure 'mr_hedgehog' is built.");
        m_statusLabel->setText("Error: Backend not found");
        m_analyzeBtn->setEnabled(true);
        return;
    }
    
    // Run analysis
    m_analysisProcess = new QProcess(this);
    connect(m_analysisProcess, &QProcess::readyReadStandardOutput, 
            this, &MainWindow::onAnalysisOutput);
    connect(m_analysisProcess, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this, &MainWindow::onAnalysisFinished);
    
    QStringList args;
    args << "--workspace" << (m_currentFolder + "/Cargo.toml");
    args << "--output" << "/tmp/mr_hedgehog_output.dot";
    args << "--engine" << "syn";
    
    m_analysisProcess->start(backendPath, args);
}

void MainWindow::onAnalysisOutput()
{
    QString output = m_analysisProcess->readAllStandardOutput();
    // Could display progress in status bar
}

void MainWindow::onAnalysisFinished(int exitCode, QProcess::ExitStatus status)
{
    m_analyzeBtn->setEnabled(true);
    
    if (exitCode == 0) {
        // Load and display results
        QString dotFile = "/tmp/mr_hedgehog_output.dot";
        if (QFile::exists(dotFile)) {
            m_graphView->loadDotFile(dotFile);
            m_statusLabel->setText("Analysis complete!");
        } else {
            m_graphView->showPlaceholder("Analysis completed but no output generated.");
            m_statusLabel->setText("No output generated");
        }
    } else {
        QString error = m_analysisProcess->readAllStandardError();
        m_graphView->showPlaceholder("Analysis failed:\n" + error);
        m_statusLabel->setText("Analysis failed");
    }
    
    m_analysisProcess->deleteLater();
    m_analysisProcess = nullptr;
}

void MainWindow::clearResults()
{
    m_graphView->clear();
    m_statusLabel->setText("Results cleared");
}

void MainWindow::showAbout()
{
    QMessageBox::about(this, "About Mr. Hedgehog",
        "<h2>Mr. Hedgehog v0.4.0</h2>"
        "<p>Rust Static Analysis Tool for Multi-Crate Workspaces</p>"
        "<p>Features:<ul>"
        "<li>Call graph generation</li>"
        "<li>AST analysis</li>"
        "<li>Dependency tracing</li>"
        "<li>SCIP semantic analysis</li>"
        "</ul></p>"
        "<p>© 2026 Frank Chen - MIT License</p>"
    );
}

void MainWindow::loadSettings()
{
    QSettings settings("Mr. Hedgehog", "Mr. HedgehogUI");
    restoreGeometry(settings.value("geometry").toByteArray());
    m_currentFolder = settings.value("lastFolder").toString();
    if (!m_currentFolder.isEmpty()) {
        m_folderPath->setText(m_currentFolder);
        updateAnalyzeButton();
    }
}

void MainWindow::saveSettings()
{
    QSettings settings("Mr. Hedgehog", "Mr. HedgehogUI");
    settings.setValue("geometry", saveGeometry());
    settings.setValue("lastFolder", m_currentFolder);
}
