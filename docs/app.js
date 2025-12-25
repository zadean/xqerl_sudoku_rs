const STORAGE_KEY = 'sudoku_games';

let wasm = null;
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

// DOM Elements - will be initialized in DOMContentLoaded
let sudokuGrid = null;
let timerDisplay = null;
let pauseBtn = null;
let resumeBtn = null;
let pauseOverlay = null;
let fillCandidatesBtn = null;
let newPuzzleBtn = null;
let statsBtn = null;
let pencilBtn = null;
let inkBtn = null;
let closeStatsBtn = null;
let statsPanel = null;
let difficultyDisplay = null;
let completionOverlay = null;
let nextPuzzleBtn = null;
let completionTime = null;
let numberIndicators = null;
let isGameCompleted = false;

// Initialize WASM and load game
document.addEventListener('DOMContentLoaded', async () => {
    try {
        console.log('DOMContentLoaded fired');
        
        // Initialize all DOM elements
        sudokuGrid = document.getElementById('sudokuGrid');
        console.log('sudokuGrid:', sudokuGrid);
        timerDisplay = document.getElementById('timer');
        pauseBtn = document.getElementById('pauseBtn');
        resumeBtn = document.getElementById('resumeBtn');
        pauseOverlay = document.getElementById('pauseOverlay');
        fillCandidatesBtn = document.getElementById('fillCandidatesBtn');
        newPuzzleBtn = document.getElementById('newPuzzleBtn');
        statsBtn = document.getElementById('statsBtn');
        pencilBtn = document.getElementById('pencilBtn');
        inkBtn = document.getElementById('inkBtn');
        closeStatsBtn = document.getElementById('closeStatsBtn');
        statsPanel = document.getElementById('statsPanel');
        difficultyDisplay = document.getElementById('difficulty');
        completionOverlay = document.getElementById('completionOverlay');
        nextPuzzleBtn = document.getElementById('nextPuzzleBtn');
        completionTime = document.getElementById('completionTime');
        
        console.log('DOM elements initialized');
        
        // Import and initialize WASM module
        console.log('Importing WASM module...');
        const wasmInit = await import('./pkg/xqerl_sudoku.js');
        console.log('WASM module imported, initializing...');
        // Call the default export (init function) to initialize WASM
        await wasmInit.default();
        console.log('WASM initialized');
        // Now we can use the exported functions
        wasm = wasmInit;
        console.log('wasm assigned:', typeof wasm);
        
        setupEventListeners();
        console.log('Event listeners set up');
        loadGameHistory();
        console.log('Game history loaded');
        loadNewPuzzle();
        console.log('New puzzle loaded');
        setupKeyboardShortcuts();
        console.log('Keyboard shortcuts set up');
    } catch (error) {
        console.error('Failed to initialize:', error);
        alert('Failed to load Sudoku puzzle engine. Please refresh the page.');
    }
});

function setupEventListeners() {
    // Query number indicators here to ensure DOM is ready
    numberIndicators = document.querySelectorAll('.number-indicator');
    
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
    
    // Add click handlers for number indicators
    numberIndicators.forEach(indicator => {
        indicator.addEventListener('click', () => {
            const num = parseInt(indicator.dataset.number);
            handleNumberInput(num);
        });
    });

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

function loadNewPuzzle() {
    try {
        if (!wasm) {
            alert('WASM engine not initialized. Please refresh the page.');
            return;
        }
        
        pauseBtn.textContent = '⏸️ Pause';
        pauseOverlay.classList.add('hidden');
        completionOverlay.classList.add('hidden');
        isPaused = false;
        elapsedTime = 0;
        candidatesPreFilled = false;
        isGameCompleted = false;
        fillCandidatesBtn.classList.remove('active');
        stopTimer();

        // Generate puzzle using WASM
        const puzzleJson = wasm.generate_puzzle();
        const puzzle = JSON.parse(puzzleJson);
        
        currentPuzzle = puzzle;
        currentSolution = puzzle.solved_board;
        initializeBoard();
        startTimer();
    } catch (error) {
        console.error('Error loading puzzle:', error);
        alert('Failed to generate puzzle: ' + (error?.message || error));
    }
}

function initializeBoard() {
    if (!currentPuzzle) return;

    console.log('initializeBoard called with puzzle:', currentPuzzle);
    
    // Initialize game board
    gameBoard = currentPuzzle.puzzle_hints.split('').map((char, idx) => ({
        value: char === '0' ? null : parseInt(char),
        isClue: char !== '0',
        pencilMarks: new Set(),
        index: idx
    }));

    console.log('gameBoard initialized with', gameBoard.length, 'cells');
    renderBoard();
    updateNumberTracker();
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
    console.log('renderBoard called, sudokuGrid:', sudokuGrid, 'gameBoard:', gameBoard);
    if (!sudokuGrid || !gameBoard) {
        console.error('renderBoard: sudokuGrid or gameBoard is null!');
        return;
    }
    
    sudokuGrid.innerHTML = '';
    console.log('sudokuGrid cleared, rendering', gameBoard.length, 'cells');
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
    updateNumberTracker();
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
    updateNumberTracker();
    selectCell(idx);
}

function updateNumberTracker() {
    if (!gameBoard || !currentSolution) return;

    // Count occurrences of each number in the solution
    const solutionCounts = {};
    for (let i = 1; i <= 9; i++) {
        solutionCounts[i] = (currentSolution.match(new RegExp(i, 'g')) || []).length;
    }

    // Count occurrences of each number in the current board
    const boardCounts = {};
    for (let i = 1; i <= 9; i++) {
        boardCounts[i] = gameBoard.filter(cell => cell.value === i).length;
    }

    // Update indicator colors
    numberIndicators.forEach(indicator => {
        const num = parseInt(indicator.dataset.number);
        if (boardCounts[num] === solutionCounts[num]) {
            // All instances of this number are placed
            indicator.classList.add('completed');
        } else {
            // Still has candidates
            indicator.classList.remove('completed');
        }
    });
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
