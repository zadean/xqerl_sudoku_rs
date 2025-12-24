const API_BASE = 'http://127.0.0.1:3000/api';
const STORAGE_KEY = 'sudoku_games';

let currentPuzzle = null;
let currentSolution = null;
let gameBoard = null;
let isPaused = false;
let timerInterval = null;
let elapsedTime = 0;
let isInkMode = false;
let gameHistory = [];
let highlightedNumber = null;
let currentSelection = null;
let undoHistory = [];
let candidatesPreFilled = false;

// DOM Elements
const sudokuGrid = document.getElementById('sudokuGrid');
const timerDisplay = document.getElementById('timer');
const pauseBtn = document.getElementById('pauseBtn');
const resumeBtn = document.getElementById('resumeBtn');
const pauseOverlay = document.getElementById('pauseOverlay');
const fillCandidatesBtn = document.getElementById('fillCandidatesBtn');
const newPuzzleBtn = document.getElementById('newPuzzleBtn');
const statsBtn = document.getElementById('statsBtn');
const pencilBtn = document.getElementById('pencilBtn');
const inkBtn = document.getElementById('inkBtn');
const closeStatsBtn = document.getElementById('closeStatsBtn');
const statsPanel = document.getElementById('statsPanel');
const difficultyDisplay = document.getElementById('difficulty');
const completionOverlay = document.getElementById('completionOverlay');
const nextPuzzleBtn = document.getElementById('nextPuzzleBtn');
const completionTime = document.getElementById('completionTime');
let isGameCompleted = false;

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    loadGameHistory();
    loadNewPuzzle();
    setupEventListeners();
    setupKeyboardShortcuts();
});

function setupEventListeners() {
    pauseBtn.addEventListener('click', togglePause);
    resumeBtn.addEventListener('click', togglePause);
    fillCandidatesBtn.addEventListener('click', fillAllCandidates);
    newPuzzleBtn.addEventListener('click', loadNewPuzzle);
    statsBtn.addEventListener('click', showStats);
    closeStatsBtn.addEventListener('click', hideStats);
    pencilBtn.addEventListener('click', () => setMode(false));
    inkBtn.addEventListener('click', () => setMode(true));
    pauseOverlay.addEventListener('click', (e) => {
        if (e.target === pauseOverlay) togglePause();
    });
    statsPanel.addEventListener('click', (e) => {
        if (e.target === statsPanel) hideStats();
    });
    nextPuzzleBtn.addEventListener('click', loadNewPuzzle);
    completionOverlay.addEventListener('click', (e) => {
        if (e.target === completionOverlay) loadNewPuzzle();
    });
}

function saveState(cellIndices) {
    if (!gameBoard) return;
    
    // Handle both single cell index and array of indices
    if (!Array.isArray(cellIndices)) {
        cellIndices = [cellIndices];
    }
    
    const savedCells = {};
    cellIndices.forEach(idx => {
        const cell = gameBoard[idx];
        savedCells[idx] = {
            value: cell.value,
            pencilMarks: new Set(cell.pencilMarks)
        };
    });
    
    undoHistory.push({
        cells: savedCells
    });
}

function undo() {
    if (undoHistory.length === 0) return;
    
    const previous = undoHistory.pop();
    
    // Restore all affected cells
    Object.entries(previous.cells).forEach(([idx, cellState]) => {
        const cell = gameBoard[parseInt(idx)];
        cell.value = cellState.value;
        cell.pencilMarks = new Set(cellState.pencilMarks);
    });
    
    renderBoard();
    if (currentSelection !== null) {
        selectCell(currentSelection);
    }
}

function setupKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
        // Handle Cmd+Z or Ctrl+Z for undo
        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'z') {
            e.preventDefault();
            undo();
            return;
        }
        
        if (isPaused) return;

        switch (e.key.toLowerCase()) {
            case 'p':
                setMode(false);
                break;
            case 'i':
                setMode(true);
                break;
            case 'c':
                fillAllCandidates();
                break;
            case ' ':
                e.preventDefault();
                togglePause();
                break;
            case 'n':
                loadNewPuzzle();
                break;
            case 's':
                isPaused ? null : showStats();
                break;
        }

        // Number keys for cell input
        const num = parseInt(e.key);
        if (num >= 1 && num <= 9) {
            handleNumberInput(num);
        }

        // Delete/Backspace for clearing
        if (e.key === 'Delete' || e.key === 'Backspace') {
            handleClearCell();
        }
    });
}

async function loadNewPuzzle() {
    try {
        pauseBtn.textContent = '⏸️ Pause';
        pauseOverlay.classList.add('hidden');
        completionOverlay.classList.add('hidden');
        isPaused = false;
        elapsedTime = 0;
        candidatesPreFilled = false;
        isGameCompleted = false;
        fillCandidatesBtn.classList.remove('active');
        stopTimer();

        const response = await fetch(`${API_BASE}/puzzles/create`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ count: 1 })
        });

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            buffer += decoder.decode(value, { stream: true });
            const lines = buffer.split('\n');
            buffer = lines.pop();

            for (const line of lines) {
                if (line.startsWith('data: ')) {
                    try {
                        const puzzle = JSON.parse(line.slice(6));
                        currentPuzzle = puzzle;
                        // Fetch full puzzle details
                        const detailResponse = await fetch(`${API_BASE}/puzzles/${puzzle.id}`);
                        const puzzleData = await detailResponse.json();
                        currentSolution = puzzleData.solved_board;
                        initializeBoard();
                        startTimer();
                    } catch (e) {
                        console.error('Error parsing puzzle:', e);
                    }
                }
            }
        }
    } catch (error) {
        console.error('Error loading puzzle:', error);
        alert('Failed to load puzzle');
    }
}

function initializeBoard() {
    if (!currentPuzzle) return;

    // Initialize game board
    gameBoard = currentPuzzle.puzzle_hints.split('').map((char, idx) => ({
        value: char === '0' ? null : parseInt(char),
        isClue: char !== '0',
        pencilMarks: new Set(),
        index: idx
    }));

    renderBoard();
    difficultyDisplay.textContent = `Difficulty: ${currentPuzzle.difficulty}`;
}

function fillAllCandidates() {
    if (!gameBoard || candidatesPreFilled) {
        candidatesPreFilled = !candidatesPreFilled;
        // Clear all candidates if toggling off
        if (!candidatesPreFilled) {
            gameBoard.forEach(cell => {
                if (!cell.value) {
                    cell.pencilMarks.clear();
                }
            });
            fillCandidatesBtn.classList.remove('active');
            renderBoard();
            return;
        }
        fillCandidatesBtn.classList.add('active');
        return;
    }

    candidatesPreFilled = true;
    fillCandidatesBtn.classList.add('active');

    // For each empty cell, add candidates 1-9 except those in same row, column, or box
    gameBoard.forEach((cell, idx) => {
        if (!cell.value && !cell.isClue) {
            const row = Math.floor(idx / 9);
            const col = idx % 9;
            
            // Get numbers already placed in row, column, and box
            const usedNumbers = new Set();
            
            // Check row
            for (let c = 0; c < 9; c++) {
                const cellValue = gameBoard[row * 9 + c].value;
                if (cellValue) usedNumbers.add(cellValue);
            }
            
            // Check column
            for (let r = 0; r < 9; r++) {
                const cellValue = gameBoard[r * 9 + col].value;
                if (cellValue) usedNumbers.add(cellValue);
            }
            
            // Check 3x3 box
            const boxRow = Math.floor(row / 3);
            const boxCol = Math.floor(col / 3);
            for (let r = boxRow * 3; r < boxRow * 3 + 3; r++) {
                for (let c = boxCol * 3; c < boxCol * 3 + 3; c++) {
                    const cellValue = gameBoard[r * 9 + c].value;
                    if (cellValue) usedNumbers.add(cellValue);
                }
            }
            
            // Add all valid candidates
            for (let num = 1; num <= 9; num++) {
                if (!usedNumbers.has(num)) {
                    cell.pencilMarks.add(num);
                }
            }
        }
    });

    renderBoard();
    if (currentSelection !== null) {
        selectCell(currentSelection);
    }
}

function renderBoard() {
    sudokuGrid.innerHTML = '';

    gameBoard.forEach((cell, idx) => {
        const cellEl = document.createElement('div');
        cellEl.className = 'sudoku-cell';
        if (cell.isClue) cellEl.classList.add('clue');
        
        // Check if the cell has an incorrect value
        if (cell.value && !cell.isClue && currentSolution) {
            const solutionChar = currentSolution[idx];
            const solutionValue = parseInt(solutionChar);
            if (cell.value !== solutionValue) {
                cellEl.classList.add('error');
            }
        }

        cellEl.dataset.index = idx;

        const content = document.createElement('div');
        content.className = 'sudoku-cell-content';

        if (cell.value) {
            const mainDiv = document.createElement('div');
            mainDiv.className = 'cell-main';
            if (highlightedNumber !== null && cell.value === highlightedNumber) {
                mainDiv.classList.add('highlighted-value');
            }
            mainDiv.textContent = cell.value;
            content.appendChild(mainDiv);
        } else if (cell.pencilMarks.size > 0) {
            const pencilDiv = document.createElement('div');
            pencilDiv.className = 'cell-pencil';
            for (let i = 1; i <= 9; i++) {
                const digit = document.createElement('div');
                digit.className = 'pencil-digit';
                if (cell.pencilMarks.has(i)) {
                    digit.textContent = i;
                    if (highlightedNumber !== null && i === highlightedNumber) {
                        digit.classList.add('highlighted-pencil');
                    }
                }
                pencilDiv.appendChild(digit);
            }
            content.appendChild(pencilDiv);
        }

        cellEl.appendChild(content);

        cellEl.addEventListener('click', () => selectCell(idx));

        sudokuGrid.appendChild(cellEl);
    });
}

function selectCell(idx) {
    if (isPaused) return;

    const cell = gameBoard[idx];
    currentSelection = idx;

    // If clicking on a cell with a value (clue or ink), highlight all instances of that number
    if (cell.value) {
        highlightedNumber = cell.value;
    } else {
        highlightedNumber = null;
    }

    // Re-render to update highlighting
    renderBoard();

    // Clear previous selections
    document.querySelectorAll('.sudoku-cell').forEach(el => {
        el.classList.remove('selected', 'related');
    });

    // Select current cell
    const cellEl = document.querySelector(`[data-index="${idx}"]`);
    if (cellEl) {
        cellEl.classList.add('selected');
    }

    // Highlight related cells (same row, column, box)
    const row = Math.floor(idx / 9);
    const col = idx % 9;
    const boxRow = Math.floor(row / 3);
    const boxCol = Math.floor(col / 3);

    gameBoard.forEach((_, i) => {
        const r = Math.floor(i / 9);
        const c = i % 9;
        const br = Math.floor(r / 3);
        const bc = Math.floor(c / 3);

        if (i !== idx && (r === row || c === col || (br === boxRow && bc === boxCol))) {
            const relatedEl = document.querySelector(`[data-index="${i}"]`);
            if (relatedEl) {
                relatedEl.classList.add('related');
            }
        }
    });
}

function handleNumberInput(num) {
    const selected = document.querySelector('.sudoku-cell.selected');
    if (!selected) return;

    const idx = parseInt(selected.dataset.index);
    const cell = gameBoard[idx];

    if (cell.isClue) return;

    if (isInkMode) {
        // Ink mode: set value directly
        // Collect all affected cells before making changes
        const affectedCells = [idx];
        const row = Math.floor(idx / 9);
        const col = idx % 9;
        const boxRow = Math.floor(row / 3);
        const boxCol = Math.floor(col / 3);
        
        gameBoard.forEach((c, i) => {
            const r = Math.floor(i / 9);
            const col_i = i % 9;
            const br = Math.floor(r / 3);
            const bc = Math.floor(col_i / 3);
            
            // Include cells that will have num removed from pencil marks
            if (i !== idx && (r === row || col_i === col || (br === boxRow && bc === boxCol))) {
                if (c.pencilMarks.has(num)) {
                    affectedCells.push(i);
                }
            }
        });
        
        // Save state of all affected cells
        saveState(affectedCells);
        
        cell.value = num;
        cell.pencilMarks.clear();
        
        // Remove this number from pencil marks in same row, column, and box
        gameBoard.forEach((c, i) => {
            const r = Math.floor(i / 9);
            const col_i = i % 9;
            const br = Math.floor(r / 3);
            const bc = Math.floor(col_i / 3);
            
            // Remove num from cells in same row, column, or box
            if (r === row || col_i === col || (br === boxRow && bc === boxCol)) {
                c.pencilMarks.delete(num);
            }
        });
    } else {
        // Pencil mode: toggle pencil mark
        saveState(idx);
        if (cell.pencilMarks.has(num)) {
            cell.pencilMarks.delete(num);
        } else {
            cell.pencilMarks.add(num);
        }
    }

    renderBoard();
    selectCell(idx);
    checkGameCompletion();
}

function handleClearCell() {
    const selected = document.querySelector('.sudoku-cell.selected');
    if (!selected) return;

    const idx = parseInt(selected.dataset.index);
    const cell = gameBoard[idx];

    if (cell.isClue) return;

    saveState(idx);
    cell.value = null;
    cell.pencilMarks.clear();

    renderBoard();
    selectCell(idx);
}

function checkGameCompletion() {
    if (isGameCompleted || !gameBoard || !currentSolution) return;

    // Check if all cells are filled
    const allFilled = gameBoard.every(cell => cell.value !== null && cell.value !== undefined);
    if (!allFilled) return;

    // Check if the solution is correct
    const isSolved = gameBoard.every((cell, idx) => {
        const solutionValue = parseInt(currentSolution[idx]);
        return cell.value === solutionValue;
    });

    if (isSolved) {
        isGameCompleted = true;
        stopTimer();
        showCompletionMessage();
    }
}

function showCompletionMessage() {
    const minutes = Math.floor(elapsedTime / 60);
    const seconds = elapsedTime % 60;
    const timeStr = `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
    completionTime.textContent = `Time: ${timeStr}`;
    completionOverlay.classList.remove('hidden');
}

function setMode(inkMode) {
    isInkMode = inkMode;
    if (inkMode) {
        pencilBtn.classList.remove('active');
        inkBtn.classList.add('active');
    } else {
        pencilBtn.classList.add('active');
        inkBtn.classList.remove('active');
    }
    renderBoard();
    // Re-apply selection if a cell was selected
    if (currentSelection !== null) {
        selectCell(currentSelection);
    }
}

function startTimer() {
    stopTimer();
    timerInterval = setInterval(() => {
        if (!isPaused) {
            elapsedTime++;
            updateTimerDisplay();
        }
    }, 1000);
}

function stopTimer() {
    if (timerInterval) clearInterval(timerInterval);
}

function updateTimerDisplay() {
    const minutes = Math.floor(elapsedTime / 60);
    const seconds = elapsedTime % 60;
    timerDisplay.textContent = `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

function togglePause() {
    isPaused = !isPaused;

    if (isPaused) {
        pauseOverlay.classList.remove('hidden');
        pauseBtn.textContent = '▶️ Resume';
    } else {
        pauseOverlay.classList.add('hidden');
        pauseBtn.textContent = '⏸️ Pause';
    }
}

function showStats() {
    const statsData = document.getElementById('statsData');
    statsData.innerHTML = '';

    if (gameHistory.length === 0) {
        statsData.innerHTML = '<div class="stat-row"><span class="stat-label">No games played yet</span></div>';
        statsPanel.classList.remove('hidden');
        return;
    }

    const stats = calculateStats();

    const rows = [
        ['Games Played', stats.gamesPlayed],
        ['Total Time', stats.totalTime],
        ['Average Time', stats.averageTime],
        ['Easiest Difficulty', stats.difficulties[0] || 'N/A'],
        ['Hardest Difficulty', stats.difficulties[stats.difficulties.length - 1] || 'N/A'],
        ['Easy Puzzles', stats.easyCount],
        ['Medium Puzzles', stats.mediumCount],
        ['Hard Puzzles', stats.hardCount],
        ['Expert Puzzles', stats.expertCount],
    ];

    rows.forEach(([label, value]) => {
        const row = document.createElement('div');
        row.className = 'stat-row';
        row.innerHTML = `<span class="stat-label">${label}</span><span class="stat-value">${value}</span>`;
        statsData.appendChild(row);
    });

    statsPanel.classList.remove('hidden');
}

function hideStats() {
    statsPanel.classList.add('hidden');
}

function calculateStats() {
    let totalTime = 0;
    let difficulties = [];
    let difficultyCount = { Easy: 0, Medium: 0, Hard: 0, Expert: 0 };

    gameHistory.forEach(game => {
        totalTime += game.time;
        difficulties.push(game.difficulty);
        difficultyCount[game.difficulty]++;
    });

    const avgSeconds = gameHistory.length > 0 ? Math.floor(totalTime / gameHistory.length) : 0;
    const avgMin = Math.floor(avgSeconds / 60);
    const avgSec = avgSeconds % 60;

    const totalMin = Math.floor(totalTime / 60);
    const totalSec = totalTime % 60;

    return {
        gamesPlayed: gameHistory.length,
        totalTime: `${totalMin}m ${totalSec}s`,
        averageTime: `${avgMin}m ${avgSec}s`,
        difficulties: difficulties.sort(),
        easyCount: difficultyCount.Easy,
        mediumCount: difficultyCount.Medium,
        hardCount: difficultyCount.Hard,
        expertCount: difficultyCount.Expert,
    };
}

function saveGameHistory() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(gameHistory));
}

function loadGameHistory() {
    const saved = localStorage.getItem(STORAGE_KEY);
    gameHistory = saved ? JSON.parse(saved) : [];
}

// Save game when leaving puzzle
window.addEventListener('beforeunload', () => {
    if (currentPuzzle && elapsedTime > 0) {
        gameHistory.push({
            puzzleId: currentPuzzle.id,
            difficulty: currentPuzzle.difficulty,
            time: elapsedTime,
            timestamp: new Date().toISOString()
        });
        saveGameHistory();
    }
});
