#include "graphview.h"

#include <QFile>
#include <QTextStream>
#include <QWheelEvent>
#include <QPainter>
#include <QRegularExpression>
#include <QDebug>
#include <QResizeEvent>
#include <cmath>

// ═══════════════════════════════════════════════════════════════════════════
// Hedgehog Implementation
// ═══════════════════════════════════════════════════════════════════════════

Hedgehog::Hedgehog(QGraphicsItem *parent)
    : QGraphicsTextItem(parent)
    , m_speed(2.0)
    , m_changeDirectionCounter(0)
    , m_facingRight(true)
{
    // Use hedgehog emoji 🦔
    setPlainText("🦔");
    
    QFont font = this->font();
    font.setPointSize(32);
    setFont(font);
    
    setZValue(100); // Always on top
    
    // Random initial velocity
    pickNewTarget();
}

void Hedgehog::setSceneBounds(const QRectF &bounds)
{
    m_bounds = bounds;
}

void Hedgehog::pickNewTarget()
{
    // Pick a random target within bounds
    if (m_bounds.isValid()) {
        qreal marginX = m_bounds.width() * 0.1;
        qreal marginY = m_bounds.height() * 0.1;
        
        m_targetPos = QPointF(
            m_bounds.left() + marginX + QRandomGenerator::global()->bounded(m_bounds.width() - 2 * marginX),
            m_bounds.top() + marginY + QRandomGenerator::global()->bounded(m_bounds.height() - 2 * marginY)
        );
    } else {
        m_targetPos = QPointF(
            QRandomGenerator::global()->bounded(500) - 250,
            QRandomGenerator::global()->bounded(400) - 200
        );
    }
    
    m_changeDirectionCounter = QRandomGenerator::global()->bounded(100, 300);
}

void Hedgehog::randomWalk()
{
    QPointF currentPos = pos();
    
    // Calculate direction to target
    QPointF direction = m_targetPos - currentPos;
    qreal distance = std::sqrt(direction.x() * direction.x() + direction.y() * direction.y());
    
    // If close to target or counter expired, pick new target
    m_changeDirectionCounter--;
    if (distance < 20 || m_changeDirectionCounter <= 0) {
        pickNewTarget();
        direction = m_targetPos - currentPos;
        distance = std::sqrt(direction.x() * direction.x() + direction.y() * direction.y());
    }
    
    // Normalize and apply speed
    if (distance > 0) {
        direction /= distance;
        
        // Add some wobble for natural movement
        qreal wobble = (QRandomGenerator::global()->bounded(100) - 50) / 100.0;
        direction.setY(direction.y() + wobble * 0.2);
        
        QPointF newPos = currentPos + direction * m_speed;
        
        // Flip hedgehog based on direction
        if (direction.x() < -0.1 && m_facingRight) {
            setScale(-1);
            m_facingRight = false;
        } else if (direction.x() > 0.1 && !m_facingRight) {
            setScale(1);
            m_facingRight = true;
        }
        
        // Boundary check
        if (m_bounds.isValid()) {
            newPos.setX(qBound(m_bounds.left(), newPos.x(), m_bounds.right() - 50));
            newPos.setY(qBound(m_bounds.top(), newPos.y(), m_bounds.bottom() - 50));
        }
        
        setPos(newPos);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GraphView Implementation
// ═══════════════════════════════════════════════════════════════════════════

GraphView::GraphView(QWidget *parent)
    : QGraphicsView(parent)
    , m_scene(nullptr)
    , m_placeholderText(nullptr)
    , m_animationTimer(nullptr)
{
    setupScene();
    
    // Enable smooth scrolling and rendering
    setRenderHint(QPainter::Antialiasing);
    setRenderHint(QPainter::TextAntialiasing);
    setRenderHint(QPainter::SmoothPixmapTransform);
    setViewportUpdateMode(QGraphicsView::FullViewportUpdate);
    setDragMode(QGraphicsView::ScrollHandDrag);
    setTransformationAnchor(QGraphicsView::AnchorUnderMouse);
    
    // Background color
    setBackgroundBrush(QColor("#11111b"));
    
    // Frame style
    setFrameShape(QFrame::NoFrame);
    
    // Setup animation timer for hedgehogs (disabled)
    m_animationTimer = new QTimer(this);
    connect(m_animationTimer, &QTimer::timeout, this, &GraphView::updateHedgehogs);
    // m_animationTimer->start(16); // Hedgehog animation disabled
    
    // Show initial placeholder
    showPlaceholder("Select a folder and click 'Run Analysis'\nto visualize the call graph");
    
    // Hedgehog spawning disabled
    // spawnHedgehogs();
}

GraphView::~GraphView()
{
    if (m_animationTimer) {
        m_animationTimer->stop();
    }
    delete m_scene;
}

void GraphView::setupScene()
{
    m_scene = new QGraphicsScene(this);
    setScene(m_scene);
}

void GraphView::spawnHedgehogs()
{
    // Create two hedgehogs
    for (int i = 0; i < 2; ++i) {
        Hedgehog *hedgehog = new Hedgehog();
        
        // Random starting position
        hedgehog->setPos(
            QRandomGenerator::global()->bounded(400) - 200,
            QRandomGenerator::global()->bounded(300) - 150
        );
        
        m_scene->addItem(hedgehog);
        m_hedgehogs.append(hedgehog);
    }
    
    // Set initial bounds
    QRectF bounds = sceneRect();
    if (!bounds.isValid() || bounds.isEmpty()) {
        bounds = QRectF(-300, -200, 600, 400);
    }
    for (Hedgehog *h : m_hedgehogs) {
        h->setSceneBounds(bounds);
    }
}

void GraphView::updateHedgehogs()
{
    for (Hedgehog *hedgehog : m_hedgehogs) {
        hedgehog->randomWalk();
    }
}

void GraphView::resizeEvent(QResizeEvent *event)
{
    QGraphicsView::resizeEvent(event);
    
    // Update hedgehog bounds when view resizes
    QRectF bounds = mapToScene(viewport()->rect()).boundingRect();
    for (Hedgehog *h : m_hedgehogs) {
        h->setSceneBounds(bounds);
    }
}

void GraphView::loadDotFile(const QString &filePath)
{
    QFile file(filePath);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        showPlaceholder("Failed to open output file:\n" + filePath);
        return;
    }
    
    QTextStream in(&file);
    QString content = in.readAll();
    file.close();
    
    parseDotFile(content);
}

void GraphView::parseDotFile(const QString &content)
{
    // Keep hedgehogs, clear everything else
    for (Hedgehog *h : m_hedgehogs) {
        m_scene->removeItem(h);
    }
    m_scene->clear();
    m_nodes.clear();
    m_placeholderText = nullptr;
    
    // Re-add hedgehogs
    for (Hedgehog *h : m_hedgehogs) {
        m_scene->addItem(h);
    }
    
    // Parse DOT format
    QStringList lines = content.split('\n');
    QList<QPair<QString, QString>> edges;
    
    QRegularExpression nodeRegex("\"([^\"]+)\"\\s*\\[label=\"([^\"]+)\"\\]");
    QRegularExpression edgeRegex("\"([^\"]+)\"\\s*->\\s*\"([^\"]+)\"");
    
    for (const QString &line : lines) {
        QRegularExpressionMatch nodeMatch = nodeRegex.match(line);
        if (nodeMatch.hasMatch()) {
            QString id = nodeMatch.captured(1);
            QString label = nodeMatch.captured(2);
            createNode(id, label);
            continue;
        }
        
        QRegularExpressionMatch edgeMatch = edgeRegex.match(line);
        if (edgeMatch.hasMatch()) {
            QString from = edgeMatch.captured(1);
            QString to = edgeMatch.captured(2);
            edges.append(qMakePair(from, to));
        }
    }
    
    layoutGraph();
    
    for (const auto &edge : edges) {
        createEdge(edge.first, edge.second);
    }
    
    if (!m_nodes.isEmpty()) {
        setSceneRect(m_scene->itemsBoundingRect().adjusted(-50, -50, 50, 50));
        fitInView(m_scene->itemsBoundingRect(), Qt::KeepAspectRatio);
        scale(0.9, 0.9);
        
        // Update hedgehog bounds
        for (Hedgehog *h : m_hedgehogs) {
            h->setSceneBounds(m_scene->itemsBoundingRect());
        }
    } else {
        showPlaceholder("No nodes found in the call graph");
    }
}

QGraphicsEllipseItem* GraphView::createNode(const QString &id, const QString &label)
{
    if (m_nodes.contains(id)) {
        return m_nodes[id];
    }
    
    QGraphicsEllipseItem *node = m_scene->addEllipse(
        0, 0, NODE_WIDTH, NODE_HEIGHT,
        QPen(QColor("#89b4fa"), 2),
        QBrush(QColor("#313244"))
    );
    
    QString displayLabel = label;
    if (displayLabel.length() > 20) {
        displayLabel = displayLabel.right(20);
        displayLabel = "..." + displayLabel;
    }
    
    QGraphicsTextItem *text = m_scene->addText(displayLabel);
    text->setDefaultTextColor(QColor("#cdd6f4"));
    text->setParentItem(node);
    
    QRectF textRect = text->boundingRect();
    text->setPos(
        (NODE_WIDTH - textRect.width()) / 2,
        (NODE_HEIGHT - textRect.height()) / 2
    );
    
    m_nodes[id] = node;
    return node;
}

void GraphView::createEdge(const QString &from, const QString &to)
{
    if (!m_nodes.contains(from) || !m_nodes.contains(to)) {
        return;
    }
    
    QGraphicsEllipseItem *fromNode = m_nodes[from];
    QGraphicsEllipseItem *toNode = m_nodes[to];
    
    QPointF fromCenter = fromNode->pos() + QPointF(NODE_WIDTH / 2, NODE_HEIGHT);
    QPointF toCenter = toNode->pos() + QPointF(NODE_WIDTH / 2, 0);
    
    QGraphicsLineItem *line = m_scene->addLine(
        QLineF(fromCenter, toCenter),
        QPen(QColor("#a6adc8"), 1.5)
    );
    line->setZValue(-1);
    
    qreal angle = std::atan2(toCenter.y() - fromCenter.y(), toCenter.x() - fromCenter.x());
    qreal arrowSize = 10;
    
    QPointF arrowP1 = toCenter - QPointF(
        std::cos(angle - M_PI / 6) * arrowSize,
        std::sin(angle - M_PI / 6) * arrowSize
    );
    QPointF arrowP2 = toCenter - QPointF(
        std::cos(angle + M_PI / 6) * arrowSize,
        std::sin(angle + M_PI / 6) * arrowSize
    );
    
    QPolygonF arrowHead;
    arrowHead << toCenter << arrowP1 << arrowP2;
    
    QGraphicsPolygonItem *arrow = m_scene->addPolygon(
        arrowHead,
        QPen(QColor("#a6adc8")),
        QBrush(QColor("#a6adc8"))
    );
    arrow->setZValue(-1);
}

void GraphView::layoutGraph()
{
    if (m_nodes.isEmpty()) return;
    
    int row = 0;
    int col = 0;
    int maxCols = 5;
    
    for (auto it = m_nodes.begin(); it != m_nodes.end(); ++it) {
        it.value()->setPos(col * NODE_SPACING_X, row * NODE_SPACING_Y);
        
        col++;
        if (col >= maxCols) {
            col = 0;
            row++;
        }
    }
}

void GraphView::showPlaceholder(const QString &message)
{
    // Keep hedgehogs, clear everything else
    for (Hedgehog *h : m_hedgehogs) {
        m_scene->removeItem(h);
    }
    m_scene->clear();
    m_nodes.clear();
    
    // Re-add hedgehogs
    for (Hedgehog *h : m_hedgehogs) {
        m_scene->addItem(h);
    }
    
    m_placeholderText = m_scene->addText(message);
    m_placeholderText->setDefaultTextColor(QColor("#6c7086"));
    
    QFont font = m_placeholderText->font();
    font.setPointSize(16);
    m_placeholderText->setFont(font);
    
    QRectF textRect = m_placeholderText->boundingRect();
    m_placeholderText->setPos(-textRect.width() / 2, -textRect.height() / 2);
    
    setSceneRect(m_scene->itemsBoundingRect().adjusted(-100, -100, 100, 100));
    
    // Update hedgehog bounds
    for (Hedgehog *h : m_hedgehogs) {
        h->setSceneBounds(sceneRect());
    }
}

void GraphView::clear()
{
    // Keep hedgehogs, clear everything else
    for (Hedgehog *h : m_hedgehogs) {
        m_scene->removeItem(h);
    }
    m_scene->clear();
    m_nodes.clear();
    m_placeholderText = nullptr;
    
    // Re-add hedgehogs
    for (Hedgehog *h : m_hedgehogs) {
        m_scene->addItem(h);
    }
}

void GraphView::wheelEvent(QWheelEvent *event)
{
    const qreal scaleFactor = 1.1;
    
    if (event->angleDelta().y() > 0) {
        scale(scaleFactor, scaleFactor);
    } else {
        scale(1 / scaleFactor, 1 / scaleFactor);
    }
}

void GraphView::drawBackground(QPainter *painter, const QRectF &rect)
{
    QGraphicsView::drawBackground(painter, rect);
    
    painter->setPen(QPen(QColor("#1e1e2e"), 0.5));
    
    qreal gridSize = 50;
    qreal left = int(rect.left()) - (int(rect.left()) % int(gridSize));
    qreal top = int(rect.top()) - (int(rect.top()) % int(gridSize));
    
    QVector<QLineF> lines;
    for (qreal x = left; x < rect.right(); x += gridSize) {
        lines.append(QLineF(x, rect.top(), x, rect.bottom()));
    }
    for (qreal y = top; y < rect.bottom(); y += gridSize) {
        lines.append(QLineF(rect.left(), y, rect.right(), y));
    }
    
    painter->drawLines(lines);
}
